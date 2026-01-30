using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

[RequirePermission(nameof(AppPermission.Container_View))]
public sealed record GetContainerInfoById(string ContainerId) : IQuery<Result<ContainerInfo>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerInfoByIdHandler(IConnectorFactory<IContainerConnector> connectorFactory, IUnitOfWork unitOfWork) : IQueryHandler<GetContainerInfoById, Result<ContainerInfo>>
{
    public async ValueTask<Result<ContainerInfo>> Handle(GetContainerInfoById query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformByContainerIdAsync(query.ContainerId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<ContainerInfo>(new NotFoundError("Platform is disconnected or unavailable."));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platform.Address,
            ContainerId: query.ContainerId
        );
        var inspectResult = await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
        if (inspectResult.IsFailure(out var error, out var inspect))
        {
            return Result.Failure<ContainerInfo>(inspectResult.Errors);
        }

        var container = await unitOfWork.Containers.GetContainerInfoAsync(query.ContainerId, cancellationToken);

        return new ContainerInfo(
              Name: inspect?.Name ?? string.Empty,
              ContainerId: inspect?.Id ?? string.Empty,
              PlatformId: platform.Id,
              PlatformName: platform.Name,
              StartedAt: inspect?.State?.StartedAt ?? string.Empty,
              FinishedAt: inspect?.State?.FinishedAt ?? string.Empty,
              Volumes: inspect?.Mounts?.Where(s => s.Name != null)?.Select(s => s.Name!)?.ToList() ?? [],
              Networks: inspect?.NetworkSettings?.Networks?.ToDictionary(n => n.Key, n => string.IsNullOrEmpty(n.Value.NetworkID) ? n.Key : n.Value.NetworkID ) ?? [],
              Ports: inspect?.HostConfig?.PortBindings ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
              State: inspect?.State?.Status ?? ContainerStateStatus.Unknown,
              Image: container?.Image,
              Deployment: container?.Deployment
            );
    }
}
