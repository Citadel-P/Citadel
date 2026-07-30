using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write, ResourceIdProperty = nameof(RemoveUserResourceAccess.UserId))]
public sealed record RemoveUserResourceAccess(
    Guid UserId,
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions) : ICommand<Result<UserDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RemoveUserResourceAccess>
    {
        public Validator()
        {
            RuleFor(x => x.UserId).NotEmpty();
            RuleFor(x => x.ResourceId).NotEmpty();
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.PermissionLevel).IsInEnum();
            RuleForEach(x => x.SpecificPermissions).IsInEnum();
            RuleFor(x => x)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage(x => $"Invalid permission: [{x.ResourceType}]-[{x.PermissionLevel}] with specifics [{string.Join(", ", x.SpecificPermissions ?? [])}] is not an allowed combination.");
        }
    }
}

internal sealed class RemoveUserResourceAccessHandler(IUnitOfWork unitOfWork, IActorResourceAccessService actorResourceAccessService)
    : ICommandHandler<RemoveUserResourceAccess, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(RemoveUserResourceAccess command, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetAsync(command.UserId, cancellationToken);
        if (user is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var actor = await unitOfWork.Actors.GetById(user.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        var result = await actorResourceAccessService.RemoveResourceAccessAsync(
            user.ActorId,
            command.ResourceType,
            command.ResourceId,
            command.PermissionLevel,
            command.SpecificPermissions,
            cancellationToken);

        if (result.IsFailure(out var error))
            return Result.Failure<UserDetails>(error);

        var persistedResourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(user.ActorId, cancellationToken);
        return new UserDetails(
            user.Id,
            user.Name,
            user.Email,
            user.ActorId,
            actor.IsEnabled,
            user.CreatedAt,
            user.CreatedByActorId,
            ResourceAccesses: persistedResourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.ResourceName, x.PermissionLevel, x.SpecificPermissions)));
    }
}
