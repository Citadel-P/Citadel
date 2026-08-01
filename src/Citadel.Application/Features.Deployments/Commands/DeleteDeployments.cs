using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Execute)]
public sealed record DeleteDeployments(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteDeploymentsHandler(
    IServiceScopeFactory scopeFactory,
    IDeploymentProcessingService deploymentProcessingService,
    IContainerProcessingService containerService,
    IPlatformStreamManager platformHub,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeleteDeploymentsHandler> logger)
    : ICommandHandler<DeleteDeployments, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(DeleteDeployments command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var requestedIds = command.Ids.Distinct().ToArray();
        var deployments = await deploymentProcessingService.MarkProcessingAsync(requestedIds, actorId, cancellationToken);

        if (deployments.Count == 0)
        {
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        // Claiming the deployments is the operation's durable commit point. From here on,
        // finish with a bounded host-lifetime token so a disconnected HTTP client cannot
        // leave rows in Processing after Docker has already removed their containers.
        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;

        var deletionCommitted = false;
        try
        {
            await deploymentProcessingService.NotifyProcessingAsync(deployments, ct: completionToken);

            var containerIds = deployments
                .Where(s => s.Container != null && s.Container?.DockerContainerId != null)
                .Select(s => s.Container!.DockerContainerId)
                .ToArray();

            if (containerIds.Length > 0)
            {
                var cmd = new Containers.Commands.DeleteContainers(containerIds, V: true, Force: true);
                var containerDeleteResult = await containerService.DeleteContainers(
                    cmd,
                    actorId,
                    completionToken,
                    claimParentResources: false);
                if (containerDeleteResult.IsFailure(out var error))
                {
                    await TryRollbackProcessingAsync(deployments);
                    return Result.Failure(error);
                }
            }

            var deleteResult = await DeleteAsync(requestedIds, actorId, completionToken);
            if (deleteResult.DeletedCount != deployments.Count)
            {
                await TryRollbackProcessingAsync(deployments);
                return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
            }

            deletionCommitted = true;
            using var postCommitCancellation = CancellationTokenSource.CreateLinkedTokenSource(
                applicationLifetime.ApplicationStopping,
                completionToken);
            postCommitCancellation.CancelAfter(TimeSpan.FromSeconds(5));
            await TryPostCommitStepAsync(
                token => deploymentProcessingService.NotifyProcessingAsync(deployments, "delete", token),
                "deployment deletion notification",
                postCommitCancellation.Token);
            foreach (var platform in deleteResult.Platforms)
            {
                await TryPostCommitStepAsync(
                    _ => platformHub.PushPlatformUpdate(platform),
                    "platform update notification",
                    postCommitCancellation.Token);
            }

            return Result.Success();
        }
        catch
        {
            if (!deletionCommitted)
                await TryRollbackProcessingAsync(deployments);
            throw;
        }
    }

    private async Task TryRollbackProcessingAsync(IReadOnlyCollection<Deployment> deployments)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await deploymentProcessingService.RollbackProcessingAsync(
                deployments,
                rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back deployment deletion claims");
        }
    }

    private async Task TryPostCommitStepAsync(
        Func<CancellationToken, Task> action,
        string step,
        CancellationToken cancellationToken)
    {
        using var stepCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            cancellationToken);
        stepCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        try
        {
            await action(stepCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed {Step} after deployment deletion committed", step);
        }
    }

    private async Task<DeleteDeploymentsResult> DeleteAsync(IEnumerable<Guid> ids, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var requestedIds = ids.Distinct().ToArray();
        var deployments = (await uow.Deployments.GetAllAsync(requestedIds, ct) ?? []).ToArray();
        if (requestedIds.Length == 0 || deployments.Length != requestedIds.Length)
            return new DeleteDeploymentsResult(0, []);

        var activityNotifications = new List<INotificationWorkItem>();
        foreach (var deployment in deployments)
        {
            var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: deployment.Id,
                platformId: deployment.PlatformId,
                resourceName: deployment.Name,
                status: ActivityStatus.Success,
                eventType: ActivityEventType.DeploymentDeleted,
                info: new DeploymentDeleted(deployment.ToSnapshot())
                );

            await uow.ActivityEventRepository.AddAsync(activity, ct);
            activityNotifications.Add(new ActivityNotificationWorkItem(
                activityHub,
                await activity.AssignActor(uow, ct)));
        }

        var platformIds = deployments.Select(deployment => deployment.PlatformId).Distinct().ToArray();
        var deleted = await uow.Deployments.RemoveRangeAsync(requestedIds, ct);
        if (deleted != deployments.Length)
        {
            await uow.RollbackAsync();
            return new DeleteDeploymentsResult(0, []);
        }

        var platforms = (await uow.Platforms.GetPlatformsWithLatestStatByIdsAsync(platformIds, ct)).ToArray();

        await uow.CommitAsync(ct);

        using var notificationCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            ct);
        notificationCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        foreach (var notification in activityNotifications)
        {
            await TryPostCommitStepAsync(
                async token => await notificationQueue.EnqueueAsync(notification, token),
                "deployment activity notification",
                notificationCancellation.Token);
        }

        return new DeleteDeploymentsResult(deleted, platforms);
    }

    private sealed record DeleteDeploymentsResult(int DeletedCount, IReadOnlyList<Domain.Entities.Platforms.Platform> Platforms);
}
