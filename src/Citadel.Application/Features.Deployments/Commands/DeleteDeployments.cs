using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Claims;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Execute)]
public sealed record DeleteDeployments(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteDeploymentsHandler(
    IServiceScopeFactory scopeFactory,
    IDeploymentProcessingService deploymentProcessingService,
    IContainerProcessingService containerService,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor)
    : ICommandHandler<DeleteDeployments, Result>
{
    public async ValueTask<Result> Handle(DeleteDeployments command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

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
        
        var deleted = await DeleteAsync(command.Ids, actorId, cancellationToken);
        if (deleted <= 0)
        {
            await deploymentProcessingService.RollbackProcessingAsync(deployments, cancellationToken);
            return Result.Failure(new NotFoundError("No deployments found matching the provided IDs for deletion."));
        }

        await deploymentProcessingService.NotifyProcessingAsync(deployments, "delete", cancellationToken);

        return Result.Success();
    }

    private async Task<int> DeleteAsync(IEnumerable<Guid> ids, Guid actorId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployments = await uow.Deployments.GetAllAsync(ids, ct);
        if (deployments is null || !deployments.Any())
            return 0;

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

        var deleted = await uow.Deployments.RemoveRangeAsync(ids, ct);
        await uow.CommitAsync(ct);

        return deleted;
    }
}

