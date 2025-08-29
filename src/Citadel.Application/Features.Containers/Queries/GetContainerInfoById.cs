using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
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
        var platform = await unitOfWork.Platforms.GetPlatformDetailsByContainerIdAsync(query.ContainerId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<ContainerInfo>(new NotFoundError("No platform found for the given ID."));
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

        return new ContainerInfo(
              Name: inspect.Name,
              ContainerId: inspect.Id,
              PlatformId: platform.Id,
              PlatformName: platform.Name,
              StartedAt: inspect.State.StartedAt,
              FinishedAt: inspect.State.FinishedAt,
              ImageName: inspect.Config?.Image,
              ImageId: inspect.Image,
              Volumes: inspect.Mounts?.Where(s => s.Name != null).Select(s => s.Name).ToList() ?? [],
              Networks: inspect.NetworkSettings?.Networks?.ToDictionary(n => n.Key, n => n.Value.NetworkID) ?? [],
              Ports: inspect.HostConfig?.PortBindings,
              State: inspect.State.Status
            );
    }
}
