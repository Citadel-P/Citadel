namespace Domain.Entities.Backups;

public sealed class BackupRestoreRun(
    Guid backupRunId,
    Guid backupRepositoryId,
    Guid targetPlatformId,
    string targetVolumeName,
    bool overwriteExisting,
    Guid triggeredByActorId,
    DateTimeOffset? queuedAt = null,
    BackupRestoreStatus status = BackupRestoreStatus.Queued,
    bool targetVolumeCreatedByCitadel = false,
    IReadOnlyList<BackupAffectedContainer>? affectedContainers = null,
    IReadOnlyList<BackupRunWarning>? warnings = null,
    DateTimeOffset? startedAt = null,
    DateTimeOffset? completedAt = null,
    int? exitCode = null,
    string? errorCode = null,
    string? errorMessage = null,
    string? targetDockerNodeId = null,
    string? targetNodeHostname = null,
    Guid? sourceBackupRunItemId = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRunId { get; private set; } = backupRunId;
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public Guid? SourceBackupRunItemId { get; private set; } = sourceBackupRunItemId;
    public BackupRestoreStatus Status { get; private set; } = status;
    public Guid TargetPlatformId { get; private set; } = targetPlatformId;
    public string? TargetDockerNodeId { get; private set; } = BackupRepository.NormalizeOptional(targetDockerNodeId);
    public string? TargetNodeHostname { get; private set; } = BackupRepository.NormalizeOptional(targetNodeHostname);
    public string TargetVolumeName { get; private set; } = BackupRepository.NormalizeName(targetVolumeName);
    public bool OverwriteExisting { get; private set; } = overwriteExisting;
    public bool TargetVolumeCreatedByCitadel { get; private set; } = targetVolumeCreatedByCitadel;
    public IReadOnlyList<BackupAffectedContainer> AffectedContainers { get; private set; } = affectedContainers ?? [];
    public IReadOnlyList<BackupRunWarning> Warnings { get; private set; } = warnings ?? [];
    public DateTimeOffset QueuedAt { get; private set; } = queuedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? StartedAt { get; private set; } = startedAt;
    public DateTimeOffset? CompletedAt { get; private set; } = completedAt;
    public int? ExitCode { get; private set; } = exitCode;
    public string? ErrorCode { get; private set; } = BackupRepository.NormalizeOptional(errorCode);
    public string? ErrorMessage { get; private set; } = BackupRepository.NormalizeOptional(errorMessage);
    public Guid TriggeredByActorId { get; private set; } = triggeredByActorId;

    public void MarkPreparing(DateTimeOffset now)
    {
        EnsureStatus(BackupRestoreStatus.Queued);
        Status = BackupRestoreStatus.Preparing;
        StartedAt ??= now.ToUniversalTime();
    }

    public void MarkRunning(DateTimeOffset now)
    {
        EnsureStatus(BackupRestoreStatus.Preparing, BackupRestoreStatus.Queued);
        Status = BackupRestoreStatus.Running;
        StartedAt ??= now.ToUniversalTime();
    }

    public void CompleteSucceeded(
        bool targetVolumeCreatedByCitadel,
        IReadOnlyList<BackupAffectedContainer> affectedContainers,
        IReadOnlyList<BackupRunWarning> warnings,
        DateTimeOffset now)
    {
        EnsureStatus(BackupRestoreStatus.Running);
        TargetVolumeCreatedByCitadel = targetVolumeCreatedByCitadel;
        AffectedContainers = affectedContainers;
        Warnings = warnings;
        Complete(warnings.Count == 0 ? BackupRestoreStatus.Succeeded : BackupRestoreStatus.SucceededWithWarnings, null, null, null, now);
    }

    public void Fail(BackupRestoreStatus status, int? exitCode, string? errorCode, string errorMessage, DateTimeOffset now)
    {
        if (status is not (BackupRestoreStatus.Failed or BackupRestoreStatus.TimedOut or BackupRestoreStatus.Rejected or BackupRestoreStatus.Interrupted))
            throw new ArgumentException("Backup restore failure status is invalid.", nameof(status));

        Complete(status, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (Status is not (BackupRestoreStatus.Queued or BackupRestoreStatus.Preparing or BackupRestoreStatus.Running))
            throw new InvalidOperationException($"Backup restore run cannot be cancelled from status {Status}.");

        Complete(BackupRestoreStatus.Cancelled, null, "backup.restore.cancelled", "Backup restore run cancelled.", now);
    }

    public static BackupRestoreRun FromPersistence(
        Guid id,
        Guid backupRunId,
        Guid backupRepositoryId,
        BackupRestoreStatus status,
        Guid targetPlatformId,
        string targetVolumeName,
        bool overwriteExisting,
        bool targetVolumeCreatedByCitadel,
        IReadOnlyList<BackupAffectedContainer> affectedContainers,
        IReadOnlyList<BackupRunWarning> warnings,
        DateTimeOffset queuedAt,
        DateTimeOffset? startedAt,
        DateTimeOffset? completedAt,
        int? exitCode,
        string? errorCode,
        string? errorMessage,
        Guid triggeredByActorId,
        string? targetDockerNodeId = null,
        string? targetNodeHostname = null,
        Guid? sourceBackupRunItemId = null)
        => new(
            backupRunId,
            backupRepositoryId,
            targetPlatformId,
            targetVolumeName,
            overwriteExisting,
            triggeredByActorId,
            queuedAt,
            status,
            targetVolumeCreatedByCitadel,
            affectedContainers,
            warnings,
            startedAt,
            completedAt,
            exitCode,
            errorCode,
            errorMessage,
            targetDockerNodeId,
            targetNodeHostname,
            sourceBackupRunItemId)
        {
            Id = id
        };

    private void Complete(BackupRestoreStatus status, int? exitCode, string? errorCode, string? errorMessage, DateTimeOffset now)
    {
        Status = status;
        ExitCode = exitCode;
        ErrorCode = BackupRepository.NormalizeOptional(errorCode);
        ErrorMessage = BackupRepository.NormalizeOptional(errorMessage);
        CompletedAt = now.ToUniversalTime();
    }

    private void EnsureStatus(params BackupRestoreStatus[] statuses)
    {
        if (!statuses.Contains(Status))
            throw new InvalidOperationException($"Backup restore run cannot transition from {Status}.");
    }
}
