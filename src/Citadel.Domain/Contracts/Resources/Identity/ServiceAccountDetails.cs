namespace Domain.Contracts.Resources.Identity;

public sealed record ServiceAccountDetails(
    Guid Id,
    string Name,
    string? Description,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    DateTime UpdatedAt,
    DateTime? ArchivedAtUtc,
    int ActiveTokenCount,
    DateTime? LastUsedAtUtc,
    IEnumerable<ResourceInfo>? Teams = null,
    IEnumerable<ResourceInfo>? Roles = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null);

public sealed record ServiceAccountTokenDetails(
    Guid Id,
    string Name,
    DateTime? ExpiresAtUtc,
    DateTime? LastUsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid? RevokedByActorId,
    Guid CreatedByActorId,
    string CreatedByName,
    DateTime CreatedAtUtc);

public sealed record ServiceAccountCredentialInfo(
    Guid CredentialId,
    Guid ServiceAccountId,
    Guid ActorId,
    string Name,
    byte[] SecretHash,
    DateTime? ExpiresAtUtc,
    DateTime? RevokedAtUtc,
    bool IsEnabled,
    DateTime? ArchivedAtUtc);

public enum ServiceAccountTokenInsertResult
{
    Created,
    AccountUnavailable,
    DuplicateName,
    ActiveLimitReached
}

public sealed record CreatedServiceAccountToken(
    ServiceAccountTokenDetails Credential,
    string Token);

public sealed record ServiceAccountLimits(
    int DefaultTokenLifetimeDays,
    int MaximumTokenLifetimeDays,
    int MaximumActiveTokensPerAccount);

public sealed record RunAsActorUsage(
    Guid Id,
    string Name,
    Hosting.Common.ResourceType ResourceType,
    bool IsActive);
