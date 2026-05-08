using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write)]
public sealed record RenameUser(Guid Id, string Name) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : AbstractValidator<RenameUser>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameUserHandler(IUnitOfWork unitOfWork) : ICommandHandler<RenameUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(RenameUser command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetUserUpdateStateAsync(command.Id, command.Name, null, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        if (state.NameExists)
            return Result.Failure<UserDetails>(new ConflictError("Name already exists"));

        state.User.UpdateMetadata(name: command.Name);
        await unitOfWork.Users.UpdateAsync(state.User, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new UserDetails(state.User.Id, state.User.Name, state.User.Email, state.User.ActorId, state.IsEnabled, state.User.CreatedAt, state.User.CreatedByActorId);
    }
}
