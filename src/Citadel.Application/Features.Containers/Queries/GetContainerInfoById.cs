using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record GetContainerInfoById(string ContainerId) : IQuery<Result<ContainerInfo>>
{
    internal class Validator : AbstractValidator<GetContainerInfoById>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerInfoByIdHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    IContainerAuthorizationService containerAuthorizationService) : IQueryHandler<GetContainerInfoById, Result<ContainerInfo>>
{
    public async ValueTask<Result<ContainerInfo>> Handle(GetContainerInfoById query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<ContainerInfo>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var container = await unitOfWork.Containers.GetByIdAsync(query.ContainerId, cancellationToken);
        if (container is null)
        {
            return Result.Failure<ContainerInfo>(new NotFoundError("Container does not exist"));
        }
        var platform = await unitOfWork.Platforms.GetByIdAsync(container.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<ContainerInfo>(new NotFoundError("Platform does not exist"));

        var inspectResult = container.DockerNodeId is not null
            ? await swarmNodeRuntimeConnector.InspectContainerAsync(
                platform,
                container.DockerNodeId,
                container.DockerContainerId,
                cancellationToken)
            : await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(
                new InspectContainerCommand(platform.Address, container.DockerContainerId),
                cancellationToken);
        if (inspectResult.IsFailure(out var error, out var inspect))
        {
            return Result.Failure<ContainerInfo>(inspectResult.Errors);
        }

        var containerInfo = await unitOfWork.Containers.GetContainerInfoByIdAsync(container.Id, cancellationToken);

        return new ContainerInfo(
              Name: inspect?.Name ?? string.Empty,
              ContainerId: inspect?.Id ?? string.Empty,
              PlatformId: platform.Id,
              PlatformName: platform.Name,
              StartedAt: inspect?.State?.StartedAt ?? string.Empty,
              FinishedAt: inspect?.State?.FinishedAt ?? string.Empty,
              Volumes: inspect?.Mounts?.Where(s => s.Name != null)?.Select(s => s.Name!)?.ToList() ?? [],
              Networks: inspect?.NetworkSettings?.Networks?.ToDictionary(n => n.Key, n => string.IsNullOrEmpty(n.Value.NetworkID) ? n.Key : n.Value.NetworkID) ?? [],
              Ports: inspect?.HostConfig?.PortBindings ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
              State: inspect?.State?.Status ?? ContainerStateStatus.Unknown,
              Image: containerInfo?.Image,
              Deployment: containerInfo?.Deployment
            );
    }
}
