namespace Infrastructure.Persistence.Dtos;

internal sealed record GitRepositoryRefDto(
    Guid Id,
    Guid GitRepositoryId,
    string Branch,
    string? ResolvedCommitSha,
    string Status,
    string? LastError,
    DateTime LastSyncedAt);
