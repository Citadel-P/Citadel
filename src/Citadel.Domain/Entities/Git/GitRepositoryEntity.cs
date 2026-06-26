using Domain.Entities.Activities;

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
    RepoWebhookConfig? webhook = null,
    RepoCommand? onClone = null,
    RepoCommand? onPull = null,
    GitRepositorySyncMode syncMode = GitRepositorySyncMode.PullInterval,
    int? syncIntervalMinutes = 5) : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = description;
    public GitReposStatus Status { get; private set; } = GitReposStatus.Created;
    public string Url { get; private set; } = Normalize(url);
    public string? DefaultBranch { get; private set; } = defaultBranch;
    public Guid? GitAccountId { get; private set; } = gitAccountId;
    public GitAccount? GitAccount { get; private set; } = null!;
    public RepoWebhookConfig? Webhook { get; private set; } = webhook;
    public RepoCommand? OnClone { get; private set; } = onClone;
    public RepoCommand? OnPull { get; private set; } = onPull;
    public GitRepositorySyncMode SyncMode { get; private set; } = syncMode;
    public int? SyncIntervalMinutes { get; private set; } = NormalizeSyncInterval(syncMode, syncIntervalMinutes);

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

    public ActivityEvent? LatestActivityEvent { get; private set; } = null;

    // Helper for the RepoCache path
    public string GetCachePath()
        => $"/app/data/repos/{Id:D}";

    public void PartialUpdate(
        string? name = null,
        string? description = null,
        string? defaultBranch = null,
        RepoWebhookConfig? webhook = null,
        RepoCommand? onClone = null,
        RepoCommand? onPull = null,
        GitRepositorySyncMode? syncMode = null,
        int? syncIntervalMinutes = null,
        GitReposStatus? status = null,
        ResourceControlState? resourceControlState = null,
        long? controlStartedAt = null,
        Guid? controlTriggeredBy = null,
        long? rowVersion = null)
    {
        if (name is not null)
            Name = name;

        if (description is not null)
            Description = description;

        if (defaultBranch is not null)
            DefaultBranch = defaultBranch;

        if (webhook is not null)
            Webhook = webhook;

        if (onClone is not null)
            OnClone = onClone;

        if (onPull is not null)
            OnPull = onPull;

        if (syncMode is not null)
        {
            SyncMode = syncMode.Value;
            SyncIntervalMinutes = NormalizeSyncInterval(syncMode.Value, syncIntervalMinutes);
        }

        if (resourceControlState is not null)
            ControlState = resourceControlState.Value;

        if (controlStartedAt is not null)
            ControlStartedAt = controlStartedAt;

        if (controlTriggeredBy is not null)
            ControlTriggeredBy = controlTriggeredBy;

        if (rowVersion is not null)
            RowVersion = rowVersion.Value;

        if (status is not null)
            Status = status.Value;
    }

    public void AssignActivityEvent(ActivityEvent activityEvent)
    {
        LatestActivityEvent = activityEvent;
    }

    public void UpdateSyncPolicy(GitRepositorySyncMode syncMode, int? syncIntervalMinutes)
    {
        SyncMode = syncMode;
        SyncIntervalMinutes = NormalizeSyncInterval(syncMode, syncIntervalMinutes);
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
        RepoWebhookConfig? webhook = null,
        RepoCommand? onClone = null,
        RepoCommand? onPull = null,
        GitRepositorySyncMode syncMode = GitRepositorySyncMode.PullInterval,
        int? syncIntervalMinutes = 5,
        ResourceControlState controlState = ResourceControlState.Idle,
        long? controlStartedAt = null,
        Guid? controlTriggeredBy = null,
        long rowVersion = 0,
        GitAccount? gitAccount = null,
        ActivityEvent? latestActivityEvent = null)
    {
        return new GitRepository(
            name,
            description,
            url,
            defaultBranch,
            gitAccountId,
            createdByActorId,
            webhook,
            onClone,
            onPull,
            syncMode,
            syncIntervalMinutes)
        {
            Id = id,
            CreatedAt = createdAt,
            GitAccount = gitAccount,
            Status = status,
            ControlState = controlState,
            ControlStartedAt = controlStartedAt,
            ControlTriggeredBy = controlTriggeredBy,
            RowVersion = rowVersion,
            LatestActivityEvent = latestActivityEvent,
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

    private static int? NormalizeSyncInterval(GitRepositorySyncMode syncMode, int? syncIntervalMinutes)
    {
        if (syncMode != GitRepositorySyncMode.PullInterval)
            return null;

        return Math.Max(1, syncIntervalMinutes ?? 5);
    }

}

public record RepoCommand(List<string> Commands, string Path = "./");
