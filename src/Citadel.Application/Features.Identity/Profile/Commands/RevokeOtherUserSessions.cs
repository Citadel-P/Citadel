using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Commands;

public sealed record RevokeOtherUserSessions : ICommand<Result<RevokeOtherUserSessionsResult>>;

public sealed record RevokeOtherUserSessionsResult(int Count);

internal sealed class RevokeOtherUserSessionsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver)
    : ICommandHandler<RevokeOtherUserSessions, Result<RevokeOtherUserSessionsResult>>
{
    public async ValueTask<Result<RevokeOtherUserSessionsResult>> Handle(RevokeOtherUserSessions command, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        var actorId = userContext.Current.ActorId;
        if (userId == Guid.Empty || actorId == Guid.Empty)
            return Result.Failure<RevokeOtherUserSessionsResult>(new UnauthorizedError("Missing user context"));

        var currentSessionId = await currentRefreshSessionResolver.ResolveAsync(userId, unitOfWork, cancellationToken);
        if (!currentSessionId.HasValue)
            return Result.Failure<RevokeOtherUserSessionsResult>(new BadRequestError("Current refresh session could not be resolved."));

        var count = await unitOfWork.RefreshTokens.DeleteOtherTokensAsync(userId, currentSessionId.Value, cancellationToken);
        if (count > 0)
        {
            var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
            await unitOfWork.ActivityEventRepository.AddAsync(
                new ActivityEvent(
                    platformId: null,
                    resourceId: userId,
                    actorId: actorId,
                    resourceName: profile?.DisplayName ?? "Current user",
                    eventType: ActivityEventType.UserOtherSessionsRevoked,
                    status: ActivityStatus.Success,
                    info: new UserOtherSessionsRevoked(count)),
                cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success(new RevokeOtherUserSessionsResult(count));
    }
}
