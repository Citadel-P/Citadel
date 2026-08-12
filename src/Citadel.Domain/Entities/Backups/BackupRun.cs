namespace Domain.Entities.Backups;

public sealed class BackupRun(
    Guid backupPolicyId,
    Guid backupRepositoryId,
    string policyNameSnapshot,
    BackupSourceSpec sourceSnapshot,
    BackupRepositoryType repositoryTypeSnapshot,
    BackupRunTrigger trigger,
    Guid? triggerSourceId,
    Guid triggeredByActorId,
    DateTimeOffset? queuedAt = null,
    BackupRunStatus status = BackupRunStatus.Queued,
    string? resticSnapshotId = null,
    string? parentSnapshotId = null,
    BackupSnapshotAvailability snapshotAvailability = BackupSnapshotAvailability.Pending,
    long? filesProcessed = null,
    long? bytesProcessed = null,
    long? bytesAdded = null,
    IReadOnlyList<BackupRunWarning>? warnings = null,
    DateTimeOffset? startedAt = null,
    DateTimeOffset? completedAt = null,
    int? exitCode = null,
    string? errorCode = null,
    string? errorMessage = null)
{
    private IReadOnlyList<BackupRunItem> items = [];

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupPolicyId { get; private set; } = backupPolicyId;
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public string PolicyNameSnapshot { get; private set; } = BackupRepository.NormalizeName(policyNameSnapshot);
    public BackupSourceSpec SourceSnapshot { get; private set; } = sourceSnapshot;
    public BackupRepositoryType RepositoryTypeSnapshot { get; private set; } = repositoryTypeSnapshot;
    public BackupRunTrigger Trigger { get; private set; } = trigger;
    public Guid? TriggerSourceId { get; private set; } = triggerSourceId;
    public BackupRunStatus Status { get; private set; } = status;
    public string? ResticSnapshotId { get; private set; } = BackupRepository.NormalizeOptional(resticSnapshotId);
    public string? ParentSnapshotId { get; private set; } = BackupRepository.NormalizeOptional(parentSnapshotId);
    public BackupSnapshotAvailability SnapshotAvailability { get; private set; } = snapshotAvailability;
    public long? FilesProcessed { get; private set; } = filesProcessed;
    public long? BytesProcessed { get; private set; } = bytesProcessed;
    public long? BytesAdded { get; private set; } = bytesAdded;
    public IReadOnlyList<BackupRunWarning> Warnings { get; private set; } = warnings ?? [];
    public DateTimeOffset QueuedAt { get; private set; } = queuedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? StartedAt { get; private set; } = startedAt;
    public DateTimeOffset? CompletedAt { get; private set; } = completedAt;
    public int? ExitCode { get; private set; } = exitCode;
    public string? ErrorCode { get; private set; } = BackupRepository.NormalizeOptional(errorCode);
    public string? ErrorMessage { get; private set; } = BackupRepository.NormalizeOptional(errorMessage);
    public Guid TriggeredByActorId { get; private set; } = triggeredByActorId;
    public IReadOnlyList<BackupRunItem> Items => items;

    public void MarkPreparing(DateTimeOffset now)
    {
        EnsureStatus(BackupRunStatus.Queued);
        Status = BackupRunStatus.Preparing;
        StartedAt ??= now.ToUniversalTime();
    }

    public void MarkRunning(DateTimeOffset now)
    {
        EnsureStatus(BackupRunStatus.Preparing, BackupRunStatus.Queued);
        Status = BackupRunStatus.Running;
        StartedAt ??= now.ToUniversalTime();
    }

    public void MarkApplyingRetention()
    {
        EnsureStatus(BackupRunStatus.Running);
        Status = BackupRunStatus.ApplyingRetention;
    }

    public void CompleteSucceeded(
        string? resticSnapshotId,
        string? parentSnapshotId,
        long? filesProcessed,
        long? bytesProcessed,
        long? bytesAdded,
        IReadOnlyList<BackupRunWarning> warnings,
        DateTimeOffset now)
    {
        if (Status is not (BackupRunStatus.Running or BackupRunStatus.ApplyingRetention))
            throw new InvalidOperationException($"Backup run cannot complete from status {Status}.");

        ResticSnapshotId = BackupRepository.NormalizeOptional(resticSnapshotId);
        ParentSnapshotId = BackupRepository.NormalizeOptional(parentSnapshotId);
        FilesProcessed = filesProcessed;
        BytesProcessed = bytesProcessed;
        BytesAdded = bytesAdded;
        Warnings = warnings;
        SnapshotAvailability = HasSnapshot(resticSnapshotId)
            ? BackupSnapshotAvailability.Available
            : BackupSnapshotAvailability.NotCreated;
        Complete(warnings.Count == 0 ? BackupRunStatus.Succeeded : BackupRunStatus.SucceededWithWarnings, null, null, null, now);
    }

    public void Fail(BackupRunStatus status, int? exitCode, string? errorCode, string errorMessage, DateTimeOffset now)
    {
        if (status is not (BackupRunStatus.Failed or BackupRunStatus.TimedOut or BackupRunStatus.Rejected or BackupRunStatus.Interrupted))
            throw new ArgumentException("Backup run failure status is invalid.", nameof(status));

        if (SnapshotAvailability == BackupSnapshotAvailability.Pending)
            SnapshotAvailability = HasSnapshot(ResticSnapshotId)
                ? BackupSnapshotAvailability.Available
                : BackupSnapshotAvailability.NotCreated;

        Complete(status, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (Status is not (BackupRunStatus.Queued or BackupRunStatus.Preparing or BackupRunStatus.Running or BackupRunStatus.ApplyingRetention))
            throw new InvalidOperationException($"Backup run cannot be cancelled from status {Status}.");

        if (SnapshotAvailability == BackupSnapshotAvailability.Pending)
            SnapshotAvailability = HasSnapshot(ResticSnapshotId)
                ? BackupSnapshotAvailability.Available
                : BackupSnapshotAvailability.NotCreated;

        Complete(BackupRunStatus.Cancelled, null, "backup.cancelled", "Backup run cancelled.", now);
    }

    public void MarkSnapshotExpired()
    {
        if (SnapshotAvailability == BackupSnapshotAvailability.Available)
            SnapshotAvailability = BackupSnapshotAvailability.Expired;
    }

    public void MarkSnapshotMissing()
    {
        if (SnapshotAvailability == BackupSnapshotAvailability.Available)
            SnapshotAvailability = BackupSnapshotAvailability.Missing;
    }

    public static BackupRun FromPersistence(
        Guid id,
        Guid backupPolicyId,
        Guid backupRepositoryId,
        string policyNameSnapshot,
        BackupSourceSpec sourceSnapshot,
        BackupRepositoryType repositoryTypeSnapshot,
        BackupRunTrigger trigger,
        Guid? triggerSourceId,
        BackupRunStatus status,
        string? resticSnapshotId,
        string? parentSnapshotId,
        BackupSnapshotAvailability snapshotAvailability,
        long? filesProcessed,
        long? bytesProcessed,
        long? bytesAdded,
        IReadOnlyList<BackupRunWarning> warnings,
        DateTimeOffset queuedAt,
        DateTimeOffset? startedAt,
        DateTimeOffset? completedAt,
        int? exitCode,
        string? errorCode,
        string? errorMessage,
        Guid triggeredByActorId)
        => new(
            backupPolicyId,
            backupRepositoryId,
            policyNameSnapshot,
            sourceSnapshot,
            repositoryTypeSnapshot,
            trigger,
            triggerSourceId,
            triggeredByActorId,
            queuedAt,
            status,
            resticSnapshotId,
            parentSnapshotId,
            snapshotAvailability,
            filesProcessed,
            bytesProcessed,
            bytesAdded,
            warnings,
            startedAt,
            completedAt,
            exitCode,
            errorCode,
            errorMessage)
        {
            Id = id
        };

    public void AssignItems(IReadOnlyList<BackupRunItem> backupRunItems)
        => items = backupRunItems;

    private void Complete(BackupRunStatus status, int? exitCode, string? errorCode, string? errorMessage, DateTimeOffset now)
    {
        Status = status;
        ExitCode = exitCode;
        ErrorCode = BackupRepository.NormalizeOptional(errorCode);
        ErrorMessage = BackupRepository.NormalizeOptional(errorMessage);
        CompletedAt = now.ToUniversalTime();
    }

    private void EnsureStatus(params BackupRunStatus[] statuses)
    {
        if (!statuses.Contains(Status))
            throw new InvalidOperationException($"Backup run cannot transition from {Status}.");
    }

    private bool HasSnapshot(string? resticSnapshotId)
        => !string.IsNullOrWhiteSpace(resticSnapshotId)
           || items.Any(static item =>
               item.Status == BackupRunItemStatus.Succeeded
               && !string.IsNullOrWhiteSpace(item.ResticSnapshotId));
}
