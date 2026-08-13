namespace Infrastructure.Persistence.Dtos;

internal sealed record ServiceAccountDto(
    Guid Id,
    string Name,
    string? Description,
    Guid ActorId,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    DateTime UpdatedAt,
    DateTime? ArchivedAtUtc);

internal sealed record ServiceAccountDetailsDto(
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
    string Teams,
    string Roles);

internal sealed record ServiceAccountTokenDto(
    Guid Id,
    string Name,
    DateTime? ExpiresAtUtc,
    DateTime? LastUsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid? RevokedByActorId,
    Guid CreatedByActorId,
    string CreatedByName,
    DateTime CreatedAtUtc);

internal sealed record ServiceAccountCredentialDto(
    Guid CredentialId,
    Guid ServiceAccountId,
    Guid ActorId,
    string Name,
    byte[] SecretHash,
    DateTime? ExpiresAtUtc,
    DateTime? RevokedAtUtc,
    bool IsEnabled,
    DateTime? ArchivedAtUtc);
