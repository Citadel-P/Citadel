using Application.Features.Identity.Profile;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Queries;

public sealed record ListUserSessions : IQuery<Result<UserSessionsDetails>>;

internal sealed class ListUserSessionsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver)
    : IQueryHandler<ListUserSessions, Result<UserSessionsDetails>>
{
    public async ValueTask<Result<UserSessionsDetails>> Handle(ListUserSessions query, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        if (userId == Guid.Empty)
            return Result.Failure<UserSessionsDetails>(new UnauthorizedError("Missing user context"));

        var currentSessionId = await currentRefreshSessionResolver.ResolveAsync(userId, unitOfWork, cancellationToken);
        var sessions = await unitOfWork.RefreshTokens.GetActiveSessionsAsync(userId, DateTime.UtcNow, cancellationToken);
        var mapped = sessions
            .Select(x => x.ToSummary(currentSessionId))
            .OrderByDescending(x => x.IsCurrent)
            .ThenByDescending(x => x.LastSeenAt)
            .ToArray();

        return Result.Success(new UserSessionsDetails(mapped, currentSessionId.HasValue));
    }
}
