using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
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
        var (deployments, containers) = await MarkDeploymentsProcessingAsync(command.DeploymentIds, cancellationToken);

        if (!containers.Any())
        {
            return Result.Failure(new NotFoundError("No containers found for the provided deployment ID (s)."));
        }

        if (!platformContainerCache.TryGetPlatformsWithContainers([.. containers.Select(s => s.DockerContainerId)], out var platformContainers))
        {
            return Result.Failure(new NotFoundError("Platform resolution failed for ID (s). Platform may be disconnected."));
        }

        await NotifyProcessingAsync(deployments, cancellationToken);

        var containerAction = ActionMap.GetValueOrDefault(command.Action, ContainerAction.START);
        foreach (var platform in platformContainers)
        {
            var result = await PatchPlatformAsync(platform, containerAction, cancellationToken);
            if (result.IsFailure())
            {
                await RollbackProcessingAsync(deployments, cancellationToken);
                return result;
            }
        }

        return Result.Success();
    }

    private async Task<(IEnumerable<Deployment>, IEnumerable<Container?>)> MarkDeploymentsProcessingAsync(IEnumerable<Guid> deploymentIds,  CancellationToken ct)
    {
        IEnumerable<Container?> containers;
        List<Deployment> successfullyUpdated = [];

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployments = await uow.Deployments.GetInfoAsync(deploymentIds, ct);

        containers = deployments?.Select(s => s.Container).Where(s => s is not null) ?? [];
        
        foreach (var deployment in deployments ?? [])
        {
            deployment.MarkProcessing();
            var affectedRow = await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true, ct);

            if (affectedRow != 0)
            {
                successfullyUpdated.Add(deployment);
            }
        }
        await uow.CommitAsync(ct);

        return (successfullyUpdated, containers);
    }

    private async Task RollbackProcessingAsync(IEnumerable<Deployment> deployments, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var deployment in deployments)
        {
            deployment.ReleaseProcessing(deployment.Status);

            await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true,
                ct);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(deployments, ct);
    }

    private async Task NotifyProcessingAsync(IEnumerable<Deployment> deployments, CancellationToken ct)
    {
        foreach (var deployment in deployments)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentHub, deployment), ct);
        }
    }

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction containerAction, CancellationToken ct)
    {
        var cmd = new PatchContainerCommand
            (
                Action: containerAction,
                PlatformAddress: platform.Address,
                ContainerIds: platform.Containers.Select(s => s.Key)
            );

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return connector.PatchAsync(cmd, ct);
    }

    private static readonly IReadOnlyDictionary<DeploymentAction, ContainerAction> ActionMap =
    new Dictionary<DeploymentAction, ContainerAction>
    {
        [DeploymentAction.RESTART] = ContainerAction.RESTART,
        [DeploymentAction.PAUSE] = ContainerAction.PAUSE,
        [DeploymentAction.UNPAUSE] = ContainerAction.UNPAUSE,
        [DeploymentAction.STOP] = ContainerAction.STOP,
        [DeploymentAction.START] = ContainerAction.START
    };
}
