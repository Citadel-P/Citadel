using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Features.Deployments.Commands;

public sealed record ChangeDeploymentState(IEnumerable<Guid> DeploymentIds, DeploymentAction Action) : ICommand<Result>;

internal sealed class ChangeDeploymentStateHandler(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache, 
    IDeploymentStreamManager deploymentHub, IConnectorFactory<IContainerConnector> connectorFactory) : ICommandHandler<ChangeDeploymentState, Result>
{
    public async ValueTask<Result> Handle(ChangeDeploymentState command, CancellationToken cancellationToken)
    {
        IEnumerable<Deployment>? deployments;
        IEnumerable<Container?> containers;
        List<Deployment> successfullyUpdated = [];
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            deployments = await uow.Deployments.GetInfoAsync(command.DeploymentIds, cancellationToken);
        
            containers = deployments?.Select(s => s.Container).Where(s => s is not null) ?? [];
            if (!containers.Any()) 
            {
                return Result.Failure(new NotFoundError("No containers found for the provided deployment ID (s)."));
            }

            foreach (var original in deployments ?? [])
            {
                original.MarkProcessing();
                var affectedRow = await uow.Deployments.UpdateProcessingAsync(
                    original.Id,
                    original.Status,
                    original.ControlState,
                    original.ControlStartedAt,
                    original.RowVersion, 
                    checkRowVersion: true, cancellationToken);

                if (affectedRow != 0)
                {
                    successfullyUpdated.Add(original);
                }
            }
            await uow.CommitAsync(cancellationToken);
        }

        foreach (var deployment in successfullyUpdated)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentHub, deployment), cancellationToken);
        }

        if (!platformContainerCache.TryGetPlatformsWithContainers([.. containers.Select(s => s.DockerContainerId)], out var platformContainers))
        {
            return Result.Failure(new NotFoundError("Platform resolution failed for ID (s). Platform may be disconnected."));
        }

        var containerAction = command.Action switch
        {
            DeploymentAction.RESTART => ContainerAction.RESTART,
            DeploymentAction.PAUSE => ContainerAction.PAUSE,
            DeploymentAction.UNPAUSE => ContainerAction.UNPAUSE,
            DeploymentAction.STOP => ContainerAction.STOP,
            DeploymentAction.START => ContainerAction.START,
            _ => ContainerAction.START
        };

        foreach (var platform in platformContainers)
        {
            var cmd = new PatchContainerCommand
            (
                Action: containerAction,
                PlatformAddress: platform.Address,
                ContainerIds: platform.Containers.Select(s => s.Key)
            );
            var result = await connectorFactory.GetConnector(platform.ConnectorType).PatchAsync(cmd, cancellationToken);
            if (result.IsFailure())
            {
                return result;
            }
        }

        return Result.Success();
    }
}
