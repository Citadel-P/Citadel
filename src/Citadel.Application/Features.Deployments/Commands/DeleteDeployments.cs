using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;

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
    IUserContextAccessor userContext)
    : ICommandHandler<DeleteDeployments, Result>
{
    public async ValueTask<Result> Handle(DeleteDeployments command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var deployments = await deploymentProcessingService.MarkProcessingAsync(command.Ids, actorId, cancellationToken);

        if (deployments.Count == 0)
        {
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        await deploymentProcessingService.NotifyProcessingAsync(deployments, ct: cancellationToken);

        var containerIds = deployments
            .Where(s => s.Container != null && s.Container?.DockerContainerId != null)
            .Select(s => s.Container!.DockerContainerId)
            .ToArray();

        if (containerIds != null && containerIds.Length > 0)
        {
            var cmd = new Containers.Commands.DeleteContainers(containerIds, V: true, Force: true);
            await containerService.DeleteContainers(cmd, actorId, cancellationToken);
        }
        
        var deleteResult = await DeleteAsync(command.Ids, actorId, cancellationToken);
        if (deleteResult.DeletedCount <= 0)
        {
            await deploymentProcessingService.RollbackProcessingAsync(deployments, cancellationToken);
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        await deploymentProcessingService.NotifyProcessingAsync(deployments, "delete", cancellationToken);
        foreach (var platform in deleteResult.Platforms)
        {
            await platformHub.PushPlatformUpdate(platform);
        }

        return Result.Success();
    }

    private async Task<DeleteDeploymentsResult> DeleteAsync(IEnumerable<Guid> ids, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployments = await uow.Deployments.GetAllAsync(ids, ct);
        if (deployments is null || !deployments.Any())
            return new DeleteDeploymentsResult(0, []);

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
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct)), ct);
        }

        var platformIds = deployments.Select(deployment => deployment.PlatformId).Distinct().ToArray();
        var deleted = await uow.Deployments.RemoveRangeAsync(ids, ct);
        var platforms = (await uow.Platforms.GetPlatformsWithLatestStatByIdsAsync(platformIds, ct)).ToArray();

        await uow.CommitAsync(ct);

        return new DeleteDeploymentsResult(deleted, platforms);
    }

    private sealed record DeleteDeploymentsResult(int DeletedCount, IReadOnlyList<Domain.Entities.Platforms.Platform> Platforms);
}
