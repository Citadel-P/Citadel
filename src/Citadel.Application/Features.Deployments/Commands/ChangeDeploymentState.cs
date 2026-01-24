using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record ChangeDeploymentState(IEnumerable<Guid> DeploymentIds, DeploymentAction Action) : ICommand<Result>;

internal sealed class ChangeDeploymentStateHandler(
    IDeploymentProcessingService deploymentProcessingService,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory) : ICommandHandler<ChangeDeploymentState, Result>
{
    public async ValueTask<Result> Handle(ChangeDeploymentState command, CancellationToken cancellationToken)
    {
        var deployments = await deploymentProcessingService.MarkProcessingAsync(command.DeploymentIds, cancellationToken);

        if (deployments.Count == 0)
        {
            return Result.Failure(new NotFoundError("No deployments found for the provided deployment ID(s)."));
        }

        var containers = deployments.Select(s => s.Container).Where(s => s is not null)!.ToArray();
        if (containers.Length == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided deployment ID(s)."));
        }

        if (!platformContainerCache.TryGetPlatformsWithContainers([.. containers.Select(s => s!.DockerContainerId)], out var platformContainers))
        {
            return Result.Failure(new NotFoundError("Platform resolution failed for ID(s). Platform may be disconnected."));
        }

        await deploymentProcessingService.NotifyProcessingAsync(deployments, cancellationToken);

        var containerAction = ActionMap.GetValueOrDefault(command.Action, ContainerAction.START);
        foreach (var platform in platformContainers)
        {
            var result = await PatchPlatformAsync(platform, containerAction, cancellationToken);
            if (result.IsFailure())
            {
                await deploymentProcessingService.RollbackProcessingAsync(deployments, cancellationToken);
                return result;
            }
        }

        return Result.Success();
    }

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction containerAction, CancellationToken ct)
    {
        var cmd = new PatchContainerCommand(
            Action: containerAction,
            PlatformAddress: platform.Address,
            ContainerIds: platform.Containers.Select(s => s.Key));

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
