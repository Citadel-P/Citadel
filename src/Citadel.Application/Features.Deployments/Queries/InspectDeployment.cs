using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Inspect)]
public sealed record InspectDeployment(Guid Id) : IQuery<Result<ContainerInspectionInfo>>;

internal sealed class InspectDeploymentHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory) : IQueryHandler<InspectDeployment, Result<ContainerInspectionInfo>>
{
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectDeployment query, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(query.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Container does not exist"));
        }

        ;
        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Platform for the container does not exist or disconnected"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platform.Address,
            ContainerId: containerId
        );
        return await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
    }
}
