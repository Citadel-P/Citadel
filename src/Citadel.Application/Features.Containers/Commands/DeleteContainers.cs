using Application.Services;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Domain.Contracts.Interfaces;

namespace Application.Features.Containers.Commands;

public sealed record DeleteContainers(string[] ContainerIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerIds).NotEmpty().WithMessage("At least one container ID must be provided.");
            RuleForEach(s => s.ContainerIds).ValidContainerId();
        }
    }
}

internal sealed class DeleteContainersHandler(
    IUserContextAccessor userContext,
    IUnitOfWork unitOfWork,
    IContainerProcessingService containerService,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken ct)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync(request.ContainerIds, ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.None, ct);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Execute] on [Platform]"));
        }

        var protectionError = await SystemContainerProtection.GetErrorAsync(unitOfWork, request.ContainerIds, ct);
        if (protectionError is not null)
        {
            return Result.Failure(new ConflictError(protectionError));
        }

        var requestedContainers = (await unitOfWork.Containers.GetByIdsAsync(request.ContainerIds, ct)).ToArray();
        if (requestedContainers.Length == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        var runtimeRequest = request with
        {
            ContainerIds = requestedContainers
                .Select(container => container.DockerContainerId)
                .Distinct(StringComparer.Ordinal)
                .ToArray()
        };

        var actorId = userContext.Current.ActorId;
        return await containerService.DeleteContainers(runtimeRequest, actorId, ct);
    }

}
