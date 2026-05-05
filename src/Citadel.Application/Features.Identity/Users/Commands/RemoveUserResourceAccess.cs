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

[RequirePermission(ResourceType.User, ResourceAction.Update)]
public sealed record RemoveUserResourceAccess(Guid UserId, ResourceType ResourceType, Guid ResourceId, ResourceAction Action) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : AbstractValidator<RemoveUserResourceAccess>
    {
        public Validator()
        {
            RuleFor(x => x.UserId).NotEmpty();
            RuleFor(x => x.ResourceId).NotEmpty();
            RuleFor(x => x.ResourceType).IsInEnum();
            RuleFor(x => x.Action).IsInEnum();
            RuleFor(x => x)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.Action))
                .WithMessage(x => $"Invalid permission: [{x.ResourceType}]-[{x.Action}] is not an allowed combination.");
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
            command.Action,
            cancellationToken);

        if (result.IsFailure(out var error))
            return Result.Failure<UserDetails>(error);

        return new UserDetails(user.Id, user.Name, user.Email, user.ActorId, actor.IsEnabled, user.CreatedAt, user.CreatedByActorId);
    }
}
