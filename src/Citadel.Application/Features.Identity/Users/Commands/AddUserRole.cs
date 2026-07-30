using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Application.Services.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write, ResourceIdProperty = nameof(AddUserRole.UserId))]
public sealed record AddUserRole(Guid UserId, Guid RoleId) : ICommand<Result<UserDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<AddUserRole>
    {
        public Validator()
        {
            RuleFor(x => x.UserId).NotEmpty();
            RuleFor(x => x.RoleId).NotEmpty();
        }
    }
}

internal sealed class AddUserRoleHandler(IUnitOfWork unitOfWork, IActorRoleService actorRoleService) : ICommandHandler<AddUserRole, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(AddUserRole command, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.GetAsync(command.UserId, cancellationToken);
        if (user is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var actor = await unitOfWork.Actors.GetById(user.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        var result = await actorRoleService.AssignRoleAsync(user.ActorId, command.RoleId, cancellationToken);
        if (result.IsFailure(out var error))
            return Result.Failure<UserDetails>(error);

        return new UserDetails(user.Id, user.Name, user.Email, user.ActorId, actor.IsEnabled, user.CreatedAt, user.CreatedByActorId);
    }
}
