namespace Domain.Entities.Git;

public class GitRepository(
    string name,
    string? description,
    string url,
    string defaultBranch,
    GitReposStatus status,
    Guid? gitAccountId,
    Guid createdByActorId) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public GitReposStatus Status { get; private set; } = status;
    public string Url { get; private set; } = Normalize(url);
    public string? DefaultBranch { get; private set; } = defaultBranch;
    public Guid? GitAccountId { get; private set; } = gitAccountId;
    public GitAccount? GitAccount { get; private set; } = null!;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    // Helper for the RepoCache path
    public string GetCachePath() => $"/data/repos/{Id}";

    public void UpdateMetadata(string name, string defaultBranch)
    {
        Name = name;
        DefaultBranch = defaultBranch;
    }

    public void UpdateSource(string url, Guid? gitAccountId)
    {
        Url = Normalize(url);
        GitAccountId = gitAccountId;
    }

    public static GitRepository FromPersistence(
        Guid id,
        string name,
        string? description,
        string url,
        string defaultBranch,
        GitReposStatus status,
        Guid? gitAccountId,
        DateTime createdAt,
        Guid createdByActorId,
        GitAccount? gitAccount = null)
    {
        return new GitRepository(name, description, url, defaultBranch, status, gitAccountId, createdByActorId)
        {
            Id = id,
            CreatedAt = createdAt,
            GitAccount = gitAccount
        };
    }

    private static string Normalize(string url)
    {
        if (string.IsNullOrWhiteSpace(url))
            throw new ArgumentException("URL cannot be empty", nameof(url));

        var normalizedUrl = url.Trim().TrimEnd('/');
        return normalizedUrl.EndsWith(".git", StringComparison.OrdinalIgnoreCase)
            ? normalizedUrl[..^4]
            : normalizedUrl;
    }
}