namespace Domain.Entities.Backups;

public sealed class BackupRunItem(
    Guid backupRunId,
    Guid platformId,
    string volumeName,
    BackupRunItemStatus status = BackupRunItemStatus.Pending,
    string? resticSnapshotId = null,
    string? parentSnapshotId = null,
    long? filesProcessed = null,
    long? bytesProcessed = null,
    long? bytesAdded = null,
    DateTimeOffset? startedAt = null,
    DateTimeOffset? completedAt = null,
    int? exitCode = null,
    string? errorCode = null,
    string? errorMessage = null,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null,
    string? dockerNodeId = null,
    string? nodeHostname = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRunId { get; private set; } = backupRunId;
    public Guid PlatformId { get; private set; } = platformId;
    public string? DockerNodeId { get; private set; } = BackupRepository.NormalizeOptional(dockerNodeId);
    public string? NodeHostname { get; private set; } = BackupRepository.NormalizeOptional(nodeHostname);
    public string VolumeName { get; private set; } = BackupRepository.NormalizeName(volumeName);
    public BackupRunItemStatus Status { get; private set; } = status;
    public string? ResticSnapshotId { get; private set; } = BackupRepository.NormalizeOptional(resticSnapshotId);
    public string? ParentSnapshotId { get; private set; } = BackupRepository.NormalizeOptional(parentSnapshotId);
    public long? FilesProcessed { get; private set; } = filesProcessed;
    public long? BytesProcessed { get; private set; } = bytesProcessed;
    public long? BytesAdded { get; private set; } = bytesAdded;
    public DateTimeOffset? StartedAt { get; private set; } = startedAt?.ToUniversalTime();
    public DateTimeOffset? CompletedAt { get; private set; } = completedAt?.ToUniversalTime();
    public int? ExitCode { get; private set; } = exitCode;
    public string? ErrorCode { get; private set; } = BackupRepository.NormalizeOptional(errorCode);
    public string? ErrorMessage { get; private set; } = BackupRepository.NormalizeOptional(errorMessage);
    public DateTimeOffset CreatedAt { get; private set; } = (createdAt ?? DateTimeOffset.UtcNow).ToUniversalTime();
    public DateTimeOffset UpdatedAt { get; private set; } = (updatedAt ?? DateTimeOffset.UtcNow).ToUniversalTime();

    public void MarkRunning(DateTimeOffset now)
    {
        EnsureStatus(BackupRunItemStatus.Pending);
        Status = BackupRunItemStatus.Running;
        StartedAt = now.ToUniversalTime();
        UpdatedAt = now.ToUniversalTime();
    }

    public void CompleteSucceeded(
        string? resticSnapshotId,
        string? parentSnapshotId,
        long? filesProcessed,
        long? bytesProcessed,
        long? bytesAdded,
        DateTimeOffset now)
    {
        EnsureStatus(BackupRunItemStatus.Running, BackupRunItemStatus.Pending);
        ResticSnapshotId = BackupRepository.NormalizeOptional(resticSnapshotId);
        ParentSnapshotId = BackupRepository.NormalizeOptional(parentSnapshotId);
        FilesProcessed = filesProcessed;
        BytesProcessed = bytesProcessed;
        BytesAdded = bytesAdded;
        Complete(BackupRunItemStatus.Succeeded, null, null, null, now);
    }

    public void Fail(int? exitCode, string errorCode, string errorMessage, DateTimeOffset now)
    {
        if (Status is BackupRunItemStatus.Succeeded)
            throw new InvalidOperationException("Succeeded backup run items cannot fail.");

        Complete(BackupRunItemStatus.Failed, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (Status is BackupRunItemStatus.Succeeded or BackupRunItemStatus.Failed)
            return;

        Complete(BackupRunItemStatus.Cancelled, null, "backup.cancelled", "Backup run cancelled.", now);
    }

    public static BackupRunItem FromPersistence(
        Guid id,
        Guid backupRunId,
        Guid platformId,
        string volumeName,
        BackupRunItemStatus status,
        string? resticSnapshotId,
        string? parentSnapshotId,
        long? filesProcessed,
        long? bytesProcessed,
        long? bytesAdded,
        DateTimeOffset? startedAt,
        DateTimeOffset? completedAt,
        int? exitCode,
        string? errorCode,
        string? errorMessage,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        string? dockerNodeId = null,
        string? nodeHostname = null)
        => new(
            backupRunId,
            platformId,
            volumeName,
            status,
            resticSnapshotId,
            parentSnapshotId,
            filesProcessed,
            bytesProcessed,
            bytesAdded,
            startedAt,
            completedAt,
            exitCode,
            errorCode,
            errorMessage,
            createdAt,
            updatedAt,
            dockerNodeId,
            nodeHostname)
        {
            Id = id
        };

    private void Complete(BackupRunItemStatus status, int? exitCode, string? errorCode, string? errorMessage, DateTimeOffset now)
    {
        Status = status;
        ExitCode = exitCode;
        ErrorCode = BackupRepository.NormalizeOptional(errorCode);
        ErrorMessage = BackupRepository.NormalizeOptional(errorMessage);
        CompletedAt = now.ToUniversalTime();
        UpdatedAt = now.ToUniversalTime();
    }

    private void EnsureStatus(params BackupRunItemStatus[] statuses)
    {
        if (!statuses.Contains(Status))
            throw new InvalidOperationException($"Backup run item cannot transition from {Status}.");
    }
}
