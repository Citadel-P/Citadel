using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write)]
public sealed record RenameUser(Guid Id, string Name) : ICommand<Result<UserDetails>>, IAdministratorRequest
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

internal sealed class RenameUserHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<RenameUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(RenameUser command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetUserUpdateStateAsync(command.Id, command.Name, null, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        if (state.NameExists)
            return Result.Failure<UserDetails>(new ConflictError("Name already exists"));

        var oldName = state.User.Name;
        state.User.UpdateMetadata(name: command.Name);
        await unitOfWork.Users.UpdateAsync(state.User, cancellationToken);

        if (!string.Equals(oldName, state.User.Name, StringComparison.Ordinal))
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                IdentityActivity.Create(
                    state.User.Id,
                    state.User.Name,
                    userContext.Current.ActorId,
                    ActivityEventType.UserRenamed,
                    new UserRenamed(oldName, state.User.Name)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        return new UserDetails(state.User.Id, state.User.Name, state.User.Email, state.User.ActorId, state.IsEnabled, state.User.CreatedAt, state.User.CreatedByActorId);
    }
}
