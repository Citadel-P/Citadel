using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, ResourceAction.Update)]
public sealed record PatchUser(Guid Id, JsonMergePatchDocument<PatchUserModel> Patch) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : PatchCommandValidator<PatchUser, PatchUserModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchUserModel,
                modelValidator: new PatchUserModelValidator())
        {
        }
    }

    internal sealed class PatchUserModelValidator : AbstractValidator<PatchUserModel>
    {
        public PatchUserModelValidator()
        {
            When(x => x.Email is not null, () => RuleFor(x => x.Email!).EmailAddress());
            When(x => x.Password is not null, () => RuleFor(x => x.Password!).MinimumLength(8).MaximumLength(128));
        }
    }
}

internal sealed class PatchUserHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(PatchUser command, CancellationToken cancellationToken)
    {
        var state = await unitOfWork.Users.GetUserUpdateStateAsync(command.Id, null, null, cancellationToken);
        if (state.User is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided user does not exist"));

        var actor = await unitOfWork.Actors.GetById(state.User.ActorId, cancellationToken);
        if (actor is null)
            return Result.Failure<UserDetails>(new NotFoundError("The provided actor does not exist"));

        var current = new PatchUserModel(state.User.Email, null, state.IsEnabled);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchUserModel);
        var emailChanged = patched.Email is not null && !string.Equals(patched.Email, state.User.Email, StringComparison.OrdinalIgnoreCase);

        if (emailChanged)
        {
            var conflicts = await unitOfWork.Users.GetConflictsAsync(state.User.Name, patched.Email!, state.User.Id, cancellationToken);
            if (conflicts.EmailExists)
                return Result.Failure<UserDetails>(new ConflictError("Email already exists"));
        }

        state.User.UpdateMetadata(email: patched.Email);
        if (patched.Password is not null)
            state.User.SetPassword(patched.Password);

        if (patched.IsEnabled.HasValue && patched.IsEnabled.Value != actor.IsEnabled)
        {
            var setEnabledResult = actor.SetEnabled(patched.IsEnabled.Value);
            if (setEnabledResult.IsFailure(out var error))
                return Result.Failure<UserDetails>(error);
        }

        await unitOfWork.Users.UpdateAsync(state.User, cancellationToken);
        await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return new UserDetails(state.User.Id, state.User.Name, state.User.Email, state.User.ActorId, actor.IsEnabled, state.User.CreatedAt, state.User.CreatedByActorId);
    }
}
