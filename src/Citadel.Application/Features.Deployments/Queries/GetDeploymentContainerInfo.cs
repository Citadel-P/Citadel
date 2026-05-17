using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentContainerInfo(Guid Id) : IQuery<Result<ContainerInfo>>;

internal class GetDeploymentContainerInfoHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory) : IQueryHandler<GetDeploymentContainerInfo, Result<ContainerInfo>>
{
    public async ValueTask<Result<ContainerInfo>> Handle(GetDeploymentContainerInfo query, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(query.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure<ContainerInfo>(new NotFoundError("Container does not exist"));
        }

        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
        {
            return Result.Failure<ContainerInfo>(new NotFoundError("Container does not exist"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platform.Address,
            ContainerId: containerId
        );
        var inspectResult = await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
        if (inspectResult.IsFailure(out var error, out var inspect))
        {
            return Result.Failure<ContainerInfo>(inspectResult.Errors);
        }

        var container = await unitOfWork.Containers.GetContainerInfoAsync(containerId, cancellationToken);

        return new ContainerInfo(
              Name: inspect?.Name ?? string.Empty,
              ContainerId: inspect?.Id ?? string.Empty,
              PlatformId: platform.Id,
              PlatformName: string.Empty,
              StartedAt: inspect?.State?.StartedAt ?? string.Empty,
              FinishedAt: inspect?.State?.FinishedAt ?? string.Empty,
              Volumes: inspect?.Mounts?.Where(s => s.Name != null)?.Select(s => s.Name!)?.ToList() ?? [],
              Networks: inspect?.NetworkSettings?.Networks?.ToDictionary(n => n.Key, n => string.IsNullOrEmpty(n.Value.NetworkID) ? n.Key : n.Value.NetworkID) ?? [],
              Ports: inspect?.HostConfig?.PortBindings ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
              State: inspect?.State?.Status ?? ContainerStateStatus.Unknown,
              Image: container?.Image,
              Deployment: container?.Deployment
            );
    }
}