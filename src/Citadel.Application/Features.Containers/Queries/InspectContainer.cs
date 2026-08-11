using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record InspectContainer(string ContainerId) : IQuery<Result<ContainerInspectionInfo>>
{
    internal sealed class Validator : AbstractValidator<InspectContainer>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal sealed class InspectContainerHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    IContainerAuthorizationService containerAuthorizationService)
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<ContainerInspectionInfo>(new ForbiddenError($"Missing specific permission [Inspect] on [Platform]"));
        }

        var container = await unitOfWork.Containers.GetByIdAsync(query.ContainerId, cancellationToken);
        if (container is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Container does not exist"));
        }
        var platform = await unitOfWork.Platforms.GetByIdAsync(container.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Platform does not exist"));

        var result = container.DockerNodeId is not null
            ? await swarmNodeRuntimeConnector.InspectContainerAsync(
                platform,
                container.DockerNodeId,
                container.DockerContainerId,
                cancellationToken)
            : await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(
                new InspectContainerCommand(platform.Address, container.DockerContainerId),
                cancellationToken);
        if (!result.IsSuccess(out var inspection))
        {
            return result;
        }

        return ContainerInspectionRedactor.RedactEnvironment(inspection);
    }
}
