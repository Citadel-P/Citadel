using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Profile;

public sealed record UserSessionsView(
    IReadOnlyCollection<UserSessionSummaryView> Sessions,
    bool CanRevokeOtherSessions)
{
    internal static UserSessionsView Map(UserSessionsDetails sessions)
        => new([.. sessions.Sessions.Select(UserSessionSummaryView.Map)], sessions.CanRevokeOtherSessions);
}

public sealed record UserSessionSummaryView(
    Guid Id,
    string DisplayName,
    string? UserAgent,
    string? IpAddress,
    DateTime CreatedAt,
    DateTime LastSeenAt,
    DateTime ExpiresAt,
    bool IsCurrent)
{
    internal static UserSessionSummaryView Map(UserSessionSummary session)
        => new(
            session.Id,
            session.DisplayName,
            session.UserAgent,
            session.IpAddress,
            session.CreatedAt,
            session.LastSeenAt,
            session.ExpiresAt,
            session.IsCurrent);
}

public sealed record RevokeOtherProfileSessionsView(int Count);
