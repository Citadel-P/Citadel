using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Inspect)]
public sealed record InspectStackContainer(Guid Id, string ContainerId) : IQuery<Result<ContainerInspectionInfo>>;

internal sealed class InspectStackContainerHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : IQueryHandler<InspectStackContainer, Result<ContainerInspectionInfo>>
{
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectStackContainer query, CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Stacks.GetContainersAsync(query.Id, cancellationToken);
        var container = containers.FirstOrDefault(x => x.DockerContainerId == query.ContainerId);
        if (container is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Container does not exist in this stack"));
        }

        var platform = await unitOfWork.Platforms.GetPlatformByContainerIdAsync(container.DockerContainerId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Platform for the container does not exist or disconnected"));
        }

        var command = new InspectContainerCommand(
            PlatformAddress: platform.Address,
            ContainerId: container.DockerContainerId);

        return await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
    }
}
