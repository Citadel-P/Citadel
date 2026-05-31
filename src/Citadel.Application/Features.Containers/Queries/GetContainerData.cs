using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record GetContainerData(string ContainerId) : IQuery<Result<(DockerContainer, Guid)>>
{
    internal class Validator : AbstractValidator<GetContainerData>
    {
        public Validator() 
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetDockerContainerHandler(
    IUnitOfWork unitOfWork,
    IContainerAuthorizationService containerAuthorizationService) : IQueryHandler<GetContainerData, Result<(DockerContainer, Guid)>>
{
    public async ValueTask<Result<(DockerContainer, Guid)>> Handle(GetContainerData query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<(DockerContainer, Guid)>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var container = await unitOfWork.Containers.GetContainerInfoAsync(query.ContainerId, cancellationToken);
        var dockerContainer = container is not null ? new DockerContainer(
            Name: container.Name,
            Image: container.Image?.Name ?? string.Empty,
            Id: container.DockerContainerId,
            ImageId: container.DockerImageId,
            State: container.State,
            ControlState: container.ControlState,
            Created: container.Created,
            Stack: container.Stack,
            ContainerStat: null,
            Ports: container.Ports) : null;
        return dockerContainer is not null ? Result.Success((dockerContainer, container?.PlatformId ?? Guid.Empty)) : Result.Failure<(DockerContainer, Guid)>(new NotFoundError("Container does not exist"));
    }
}
