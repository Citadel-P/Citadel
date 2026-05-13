using Application.Services;
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
    IContainerAuthorizationService containerAuthorizationService)
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<ContainerInspectionInfo>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var platform = await unitOfWork.Platforms.GetPlatformByContainerIdAsync(query.ContainerId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError("Container does not exist"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platform.Address, 
            ContainerId: query.ContainerId
        );
        return await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
    }
}