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

internal sealed class ChangeDeploymentStateHandler(IServiceScopeFactory scopeFactory, IPlatformContainerCache platformContainerCache, IConnectorFactory<IContainerConnector> connectorFactory) : ICommandHandler<ChangeDeploymentState, Result>
{
    public async ValueTask<Result> Handle(ChangeDeploymentState command, CancellationToken cancellationToken)
    {
        IEnumerable<Container>? containers;
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            containers = await uow.Containers.GetByDeploymentIdsAsync(command.DeploymentIds, cancellationToken);
        }

        if (!containers.Any()) 
        {
            return Result.Failure(new NotFoundError("No containers found for the provided deployment ID (s)."));
        }

        var groupByPlatform = containers.GroupBy(c => c.PlatformId);

        if (!platformContainerCache.TryGetPlatformsByContainersId([.. containers.Select(s => s.DockerContainerId)], out var platformContainers))
        {
            return Result.Failure(new NotFoundError("Platform resolution failed for ID (s). Platform may be disconnected."));
        }

        var containerAction = command.Action switch
        {
            DeploymentAction.RESTART => ContainerAction.RESTART,
            DeploymentAction.PAUSE => ContainerAction.PAUSE,
            DeploymentAction.UNPAUSE => ContainerAction.UNPAUSE,
            DeploymentAction.STOP => ContainerAction.STOP,
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
