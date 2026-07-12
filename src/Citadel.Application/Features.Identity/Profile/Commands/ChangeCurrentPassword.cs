using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Commands;

public sealed record ChangeCurrentPassword(string CurrentPassword, string NewPassword) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<ChangeCurrentPassword>
    {
        public Validator()
        {
            RuleFor(x => x.CurrentPassword).NotNull().MinimumLength(6).MaximumLength(128);
            RuleFor(x => x.NewPassword).NotNull().MinimumLength(6).MaximumLength(128);
        }
    }
}

internal sealed class ChangeCurrentPasswordHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver)
    : ICommandHandler<ChangeCurrentPassword, Result>
{
    public async ValueTask<Result> Handle(ChangeCurrentPassword command, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        var actorId = userContext.Current.ActorId;
        if (userId == Guid.Empty || actorId == Guid.Empty)
            return Result.Failure(new UnauthorizedError("Missing user context"));

        var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
        if (profile is null)
            return Result.Failure(new NotFoundError("Current user does not exist"));

        if (profile.OidcProviderId.HasValue)
            return Result.Failure(new BadRequestError("Password changes are managed by the identity provider."));

        var user = await unitOfWork.Users.GetAsync(userId, cancellationToken);
        if (user is null)
            return Result.Failure(new NotFoundError("Current user does not exist"));

        var authInfo = await unitOfWork.Users.GetUserAuthInfoByIdAsync(userId, cancellationToken);
        if (authInfo?.Password is null || !User.IsValidPassword(command.CurrentPassword, authInfo.Password))
            return Result.Failure(new BadRequestError("Current password is incorrect."));

        user.SetPassword(command.NewPassword);
        await unitOfWork.Users.UpdateAsync(user, cancellationToken);

        var currentSessionId = await currentRefreshSessionResolver.ResolveAsync(userId, unitOfWork, cancellationToken);
        if (currentSessionId.HasValue)
            await unitOfWork.RefreshTokens.DeleteOtherTokensAsync(userId, currentSessionId.Value, cancellationToken);
        else
            await unitOfWork.RefreshTokens.DeleteAllTokensAsync(userId, cancellationToken);

        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: userId,
                actorId: actorId,
                resourceName: profile.DisplayName,
                eventType: ActivityEventType.UserPasswordChanged,
                status: ActivityStatus.Success,
                info: new UserPasswordChanged()),
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
