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
    public string GetCachePath()
    {
        var repositoryName = ExtractRepositoryName(Url);
        if (!string.IsNullOrWhiteSpace(repositoryName))
            return $"/app/data/repos/{repositoryName}";

        var fallbackName = SanitizeDirectoryName(Name);
        if (!string.IsNullOrWhiteSpace(fallbackName))
            return $"/app/data/repos/{fallbackName}";

        return $"/app/data/repos/{Id}";
    }

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
        Status = GitReposStatus.Pending;
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

    private static string ExtractRepositoryName(string url)
    {
        if (string.IsNullOrWhiteSpace(url))
            return string.Empty;

        string candidate;
        if (Uri.TryCreate(url, UriKind.Absolute, out var uri))
        {
            candidate = uri.AbsolutePath.TrimEnd('/');
            var separatorIndex = candidate.LastIndexOf('/');
            candidate = separatorIndex >= 0 ? candidate[(separatorIndex + 1)..] : candidate;
        }
        else
        {
            var normalized = url.Trim().TrimEnd('/');
            var separatorIndex = normalized.LastIndexOfAny(['/', ':']);
            candidate = separatorIndex >= 0 ? normalized[(separatorIndex + 1)..] : normalized;
        }

        return SanitizeDirectoryName(candidate);
    }

    private static string SanitizeDirectoryName(string value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return string.Empty;

        var result = string.Create(value.Length, value, static (span, source) =>
        {
            for (var i = 0; i < source.Length; i++)
            {
                var c = source[i];
                span[i] = char.IsLetterOrDigit(c) || c is '-' or '_' or '.' ? c : '-';
            }
        }).Trim('-');

        return result.EndsWith(".git", StringComparison.OrdinalIgnoreCase)
            ? result[..^4]
            : result;
    }
}

public record RepoCommand(List<string> Commands, string Path = "./");