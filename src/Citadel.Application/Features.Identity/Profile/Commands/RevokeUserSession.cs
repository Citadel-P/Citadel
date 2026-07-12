using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Commands;

public sealed record RevokeUserSession(Guid SessionId) : ICommand<Result>;

internal sealed class RevokeUserSessionHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver)
    : ICommandHandler<RevokeUserSession, Result>
{
    public async ValueTask<Result> Handle(RevokeUserSession command, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        var actorId = userContext.Current.ActorId;
        if (userId == Guid.Empty || actorId == Guid.Empty)
            return Result.Failure(new UnauthorizedError("Missing user context"));

        var currentSessionId = await currentRefreshSessionResolver.ResolveAsync(userId, unitOfWork, cancellationToken);
        var deleted = await unitOfWork.RefreshTokens.DeleteOwnedSessionAsync(
            command.SessionId,
            userId,
            currentSessionId,
            cancellationToken);

        if (deleted == 0)
            return Result.Failure(new NotFoundError("Session does not exist"));

        var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: userId,
                actorId: actorId,
                resourceName: profile?.DisplayName ?? "Current user",
                eventType: ActivityEventType.UserSessionRevoked,
                status: ActivityStatus.Success,
                info: new UserSessionRevoked(command.SessionId)),
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}
