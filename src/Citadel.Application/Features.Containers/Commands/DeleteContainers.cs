using Application.Services;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

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
    IContainerProcessingService containerService,
    IHttpContextAccessor httpContextAccessor,
    IContainerPlatformAuthorizationService containerPlatformAuthorizationService)
    : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken ct)
    {
        var hasAccess = await containerPlatformAuthorizationService.HasAccessAsync(request.ContainerIds, PermissionLevel.Execute, SpecificPermission.None, ct);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Execute] on [Platform]"));
        }

        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        return await containerService.DeleteContainers(request, actorId, ct);
    }

}