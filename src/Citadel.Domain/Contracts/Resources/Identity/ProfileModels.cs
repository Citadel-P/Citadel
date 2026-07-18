using Domain;

namespace Domain.Contracts.Resources.Identity;

public sealed record CurrentProfileDetails(
    Guid Id,
    string DisplayName,
    string Email,
    Guid ActorId,
    DateTime CreatedAt,
    IReadOnlyCollection<ResourceInfo> DirectRoles,
    IReadOnlyCollection<ResourceInfo> Teams,
    bool HasLocalPassword,
    Guid? OidcProviderId,
    string? OidcProviderName);

public sealed record UserPreferencesDetails(
    string? TimeZone,
    UserDateTimeFormat DateTimeFormat,
    UserTheme Theme,
    bool IsPersisted);

public sealed record PatchUserPreferencesModel(
    string? TimeZone,
    UserDateTimeFormat? DateTimeFormat,
    UserTheme? Theme);

public sealed record UserSessionRecord(
    Guid Id,
    string? UserAgent,
    string? IpAddress,
    DateTime CreatedAt,
    DateTime LastSeenAt,
    DateTime ExpiresAt);

public sealed record UserSessionSummary(
    Guid Id,
    string DisplayName,
    string? UserAgent,
    string? IpAddress,
    DateTime CreatedAt,
    DateTime LastSeenAt,
    DateTime ExpiresAt,
    bool IsCurrent);

public sealed record UserSessionsDetails(
    IReadOnlyCollection<UserSessionSummary> Sessions,
    bool CanRevokeOtherSessions);
