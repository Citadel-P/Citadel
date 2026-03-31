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
    Guid? gitAccountId,
    Guid createdByActorId,
    bool webHookEnabled = false,
    string webHookSecret = "",
    RepoCommand? onClone = null,
    RepoCommand? onPull = null) : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public GitReposStatus Status { get; private set; } = GitReposStatus.Created;
    public string Url { get; private set; } = Normalize(url);
    public string? DefaultBranch { get; private set; } = defaultBranch;
    public Guid? GitAccountId { get; private set; } = gitAccountId;
    public GitAccount? GitAccount { get; private set; } = null!;
    public bool WebHookEnabled { get; private set; } = webHookEnabled;
    public string WebHookSecret { get; private set; } = webHookSecret ?? string.Empty;
    public RepoCommand? OnClone { get; private set; } = onClone;
    public RepoCommand? OnPull { get; private set; } = onPull;

    #region IAuditedEntity Members
    public DateTime CreatedAt { get; private set; } = DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    #endregion

    #region IReconcilableResource Members
    public ResourceControlState ControlState { get; private set; } = ResourceControlState.Idle;
    public Guid? ControlTriggeredBy { get; private set; }
    public long? ControlStartedAt { get; private set; }
    public long RowVersion { get; private set; }
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
        RepoCommand? onClone = null,
        RepoCommand? onPull = null,
        ResourceControlState? resourceControlState = null,
        long? controlStartedAt = null,
        Guid? controlTriggeredBy = null,
        long? rowVersion = null)
    {
        if (webHookEnabled.HasValue)
            WebHookEnabled = webHookEnabled.Value;

        if (webHookSecret is not null)
            WebHookSecret = webHookSecret;

        if (onClone is not null)
            OnClone = onClone;

        if (onPull is not null)
            OnPull = onPull;

        if (resourceControlState is not null)
            ControlState = resourceControlState.Value;

        if (controlStartedAt is not null)
            ControlStartedAt = controlStartedAt;

        if (controlTriggeredBy is not null)
            ControlTriggeredBy = controlTriggeredBy;

        if (rowVersion is not null)
            RowVersion = rowVersion.Value;
    }

    public void UpdateSource(string url, Guid? gitAccountId)
    {
        Url = Normalize(url);
        GitAccountId = gitAccountId;
    }

    public void MarkProcessing(Guid controlTriggeredBy)
    {
        Status = GitReposStatus.Processing;
        ControlTriggeredBy = controlTriggeredBy;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
    }

    public void ReleaseProcessing(GitReposStatus status)
    {
        Status = status;
        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        ControlTriggeredBy = null;
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
        RepoCommand? onClone = null,
        RepoCommand? onPull = null,
        ResourceControlState controlState = ResourceControlState.Idle,
        long? controlStartedAt = null,
        Guid? controlTriggeredBy = null,
        long rowVersion = 0,
        GitAccount? gitAccount = null)
    {
        return new GitRepository(name, description, url, defaultBranch, gitAccountId, createdByActorId)
        {
            Id = id,
            CreatedAt = createdAt,
            GitAccount = gitAccount,
            WebHookEnabled = webHookEnabled,
            WebHookSecret = webHookSecret ?? string.Empty,
            OnClone = onClone,
            OnPull = onPull,
            Status = status,
            ControlState = controlState,
            ControlStartedAt = controlStartedAt,
            ControlTriggeredBy = controlTriggeredBy,
            RowVersion = rowVersion
        };
    }

    private static string Normalize(string url)
    {
        if (string.IsNullOrWhiteSpace(url))
            return url;

        var normalizedUrl = url.Trim().TrimEnd('/');
        return normalizedUrl.EndsWith(".git", StringComparison.OrdinalIgnoreCase)
            ? normalizedUrl[..^4]
            : normalizedUrl;
    }
}

public record RepoCommand(List<string> Commands, string Path = "./");