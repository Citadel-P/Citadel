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
public sealed record RemoveUserRole(Guid UserId, Guid RoleId) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : AbstractValidator<RemoveUserRole>
    {
        public Validator()
        {
            RuleFor(x => x.UserId).NotEmpty();
            RuleFor(x => x.RoleId).NotEmpty();
        }
    }
}

internal sealed class RemoveUserRoleHandler(IUnitOfWork unitOfWork) : ICommandHandler<RemoveUserRole, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(RemoveUserRole command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetRoleAssignmentStateAsync(command.UserId, command.RoleId, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        if (!state.RoleExists)
            return Result.Failure<UserDetails>(new NotFoundError("The provided role does not exist"));

        if (!state.HasRole)
            return Result.Failure<UserDetails>(new NotFoundError("The provided role is not assigned to the user"));

        await unitOfWork.Users.RemoveActorRoleAsync(state.User.ActorId, command.RoleId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return state.User;
    }
}
