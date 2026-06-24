namespace Domain.Entities.Git;

public sealed class GitRepositoryRef
{
    private GitRepositoryRef()
    {
    }

    public GitRepositoryRef(
        Guid gitRepositoryId,
        string branch,
        string resolvedCommitSha,
        GitReposStatus status,
        string? lastError = null)
    {
        Id = Guid.CreateVersion7();
        GitRepositoryId = gitRepositoryId;
        Branch = branch;
        ResolvedCommitSha = resolvedCommitSha;
        Status = status;
        LastError = lastError;
        LastSyncedAt = DateTime.UtcNow;
    }

    public Guid Id { get; private set; }
    public Guid GitRepositoryId { get; private set; }
    public string Branch { get; private set; } = string.Empty;
    public string? ResolvedCommitSha { get; private set; }
    public GitReposStatus Status { get; private set; }
    public string? LastError { get; private set; }
    public DateTime LastSyncedAt { get; private set; }

    public static GitRepositoryRef FromPersistence(
        Guid id,
        Guid gitRepositoryId,
        string branch,
        string? resolvedCommitSha,
        GitReposStatus status,
        string? lastError,
        DateTime lastSyncedAt)
    {
        return new GitRepositoryRef
        {
            Id = id,
            GitRepositoryId = gitRepositoryId,
            Branch = branch,
            ResolvedCommitSha = resolvedCommitSha,
            Status = status,
            LastError = lastError,
            LastSyncedAt = lastSyncedAt
        };
    }
}
