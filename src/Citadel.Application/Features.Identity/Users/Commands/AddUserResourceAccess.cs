using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write, ResourceIdProperty = nameof(AddUserResourceAccess.UserId))]
public sealed record AddUserResourceAccess(
    Guid UserId,
    ResourceType ResourceType,
    Guid ResourceId,
    PermissionLevel PermissionLevel,
    IEnumerable<SpecificPermission>? SpecificPermissions) : ICommand<Result<UserDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<AddUserResourceAccess>
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

internal sealed class AddUserResourceAccessHandler(
    IUnitOfWork unitOfWork,
    IActorResourceAccessService actorResourceAccessService,
    IUserContextAccessor userContext)
    : ICommandHandler<AddUserResourceAccess, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(AddUserResourceAccess command, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetAsync(command.UserId, cancellationToken);
        if (user is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var actor = await unitOfWork.Actors.GetById(user.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        var oldSnapshot = await IdentityActivity.CaptureUserAsync(unitOfWork, user.Id, cancellationToken);
        if (oldSnapshot is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var access = new IdentityResourceAccessSnapshot(
            command.ResourceType,
            command.ResourceId,
            command.PermissionLevel,
            Permission.ToSpecificPermissionsMask(command.SpecificPermissions));
        var newSnapshot = IdentityActivity.WithResourceAccess(oldSnapshot, access, add: true);
        await unitOfWork.ActivityEventRepository.AddAsync(
            IdentityActivity.Create(
                user.Id,
                user.Name,
                userContext.Current.ActorId,
                ActivityEventType.UserUpdated,
                new UserUpdated(oldSnapshot, newSnapshot, PasswordChanged: false)),
            cancellationToken);

        var result = await actorResourceAccessService.AddResourceAccessAsync(
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
