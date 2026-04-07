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
public sealed record AddUserRole(Guid UserId, Guid RoleId) : ICommand<Result<UserDetails>>
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

internal sealed class AddUserRoleHandler(IUnitOfWork unitOfWork) : ICommandHandler<AddUserRole, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(AddUserRole command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetRoleAssignmentStateAsync(command.UserId, command.RoleId, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        if (!state.RoleExists)
            return Result.Failure<UserDetails>(new NotFoundError("The provided role does not exist"));

        if (state.HasRole)
            return Result.Failure<UserDetails>(new ConflictError("The user already has the provided role"));

        await unitOfWork.Users.AddActorRoleAsync(state.User.ActorId, command.RoleId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return state.User;
    }
}
