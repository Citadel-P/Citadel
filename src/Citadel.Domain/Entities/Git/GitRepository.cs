namespace Domain.Entities.Git;

/// <summary>
/// Source of Truth for the entire GitOps pipeline
/// It manages the connection and the "physical" files on the server, while other resources (like Stacks) consume those files to do their jobs
/// </summary>
public class GitRepository(
    string name,
    string? description,
    string url,
    string defaultBranch,
    GitReposStatus status,
    Guid? gitAccountId,
    Guid createdByActorId,
    bool webHookEnabled = false,
    string webHookSecret = "",
    List<RepoCommand>? onClone = null,
    List<RepoCommand>? onPull = null) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public GitReposStatus Status { get; private set; } = status;
    public string Url { get; private set; } = Normalize(url);
    public string? DefaultBranch { get; private set; } = defaultBranch;
    public Guid? GitAccountId { get; private set; } = gitAccountId;
    public GitAccount? GitAccount { get; private set; } = null!;
    public bool WebHookEnabled { get; private set; } = webHookEnabled;
    public string WebHookSecret { get; private set; } = webHookSecret ?? string.Empty;
    public List<RepoCommand> OnClone { get; private set; } = onClone ?? new List<RepoCommand>();
    public List<RepoCommand> OnPull { get; private set; } = onPull ?? new List<RepoCommand>();

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    // Helper for the RepoCache path
    public string GetCachePath() => $"/data/repos/{Id}";

    public void UpdateMetadata(string name, string? description, string defaultBranch, GitReposStatus status)
    {
        Name = name;
        Description = description;
        DefaultBranch = defaultBranch;
        Status = status;
    }

    public void PartialUpdate(
        bool? webHookEnabled = null,
        string? webHookSecret = null,
        List<RepoCommand>? onClone = null,
        List<RepoCommand>? onPull = null)
    {
        if (webHookEnabled.HasValue)
            WebHookEnabled = webHookEnabled.Value;

        if (webHookSecret is not null)
            WebHookSecret = webHookSecret;

        if (onClone is not null)
            OnClone = onClone;

        if (onPull is not null)
            OnPull = onPull;
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
        bool webHookEnabled = false,
        string? webHookSecret = null,
        List<RepoCommand>? onClone = null,
        List<RepoCommand>? onPull = null,
        GitAccount? gitAccount = null)
    {
        return new GitRepository(name, description, url, defaultBranch, status, gitAccountId, createdByActorId)
        {
            Id = id,
            CreatedAt = createdAt,
            GitAccount = gitAccount,
            WebHookEnabled = webHookEnabled,
            WebHookSecret = webHookSecret ?? string.Empty,
            OnClone = onClone ?? new List<RepoCommand>(),
            OnPull = onPull ?? new List<RepoCommand>()
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

public record RepoCommand(string Command, string Path = "./");