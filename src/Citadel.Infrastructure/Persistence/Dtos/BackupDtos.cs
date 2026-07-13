namespace Infrastructure.Persistence.Dtos;

internal sealed record BackupRepositoryDto(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    string Type,
    string Spec,
    Guid PasswordSecretId,
    string Status,
    DateTime? LastPrunedAt,
    DateTime? LastCheckedAt,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    DateTime? ArchivedAt,
    long RowVersion)
{
    public BackupRepositoryDto()
        : this(Guid.Empty, string.Empty, string.Empty, null, string.Empty, "{}", Guid.Empty, string.Empty, null, null, Guid.Empty, DateTime.MinValue, DateTime.MinValue, null, 0)
    {
    }
}

internal sealed record BackupRepositoryValidationDto(
    Guid Id,
    Guid BackupRepositoryId,
    string Location,
    Guid? PlatformId,
    string Status,
    DateTime LastValidatedAt,
    string? LastErrorCode,
    string? LastErrorMessage)
{
    public BackupRepositoryValidationDto()
        : this(Guid.Empty, Guid.Empty, string.Empty, null, string.Empty, DateTime.MinValue, null, null)
    {
    }
}

internal sealed record BackupPolicyDto(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    string Source,
    Guid BackupRepositoryId,
    bool Enabled,
    string? Cron,
    string? TimeZone,
    int KeepLastSuccessful,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId,
    string ControlState,
    Guid? CurrentRunId,
    DateTime? LastScheduledRunAt,
    DateTime? FirstSuccessfulRunAt,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    DateTime? ArchivedAt,
    long RowVersion,
    string? TagsJson = null)
{
    public BackupPolicyDto()
        : this(
            Guid.Empty,
            string.Empty,
            string.Empty,
            null,
            "{}",
            Guid.Empty,
            false,
            null,
            null,
            14,
            14_400,
            true,
            Guid.Empty,
            string.Empty,
            null,
            null,
            null,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue,
            null,
            0)
    {
    }
}

internal sealed record VolumeBackupCoverageDto(
    Guid PlatformId,
    string VolumeName,
    string Status,
    int PolicyCount,
    Guid? LastRunId,
    string? LastRunStatus,
    DateTime? LastRunAt,
    DateTime? LastSuccessfulRunAt,
    DateTime? NextRunAt)
{
    public VolumeBackupCoverageDto()
        : this(Guid.Empty, string.Empty, string.Empty, 0, null, null, null, null, null)
    {
    }
}

internal sealed record BackupRunDto(
    Guid Id,
    Guid BackupPolicyId,
    Guid BackupRepositoryId,
    string PolicyNameSnapshot,
    string SourceSnapshot,
    string RepositoryTypeSnapshot,
    string Trigger,
    Guid? TriggerSourceId,
    string Status,
    string? ResticSnapshotId,
    string? ParentSnapshotId,
    string SnapshotAvailability,
    long? FilesProcessed,
    long? BytesProcessed,
    long? BytesAdded,
    string Warnings,
    DateTime QueuedAt,
    DateTime? StartedAt,
    DateTime? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid TriggeredByActorId)
{
    public BackupRunDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            "{}",
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            null,
            null,
            string.Empty,
            null,
            null,
            null,
            "[]",
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty)
    {
    }
}

internal sealed record BackupRunQueueDto(
    int ResultStatus,
    Guid? Id,
    Guid? BackupPolicyId,
    Guid? BackupRepositoryId,
    string? PolicyNameSnapshot,
    string? SourceSnapshot,
    string? RepositoryTypeSnapshot,
    string? Trigger,
    Guid? TriggerSourceId,
    string? Status,
    string? ResticSnapshotId,
    string? ParentSnapshotId,
    string? SnapshotAvailability,
    long? FilesProcessed,
    long? BytesProcessed,
    long? BytesAdded,
    string? Warnings,
    DateTime? QueuedAt,
    DateTime? StartedAt,
    DateTime? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid? TriggeredByActorId)
{
    public BackupRunQueueDto()
        : this(
            0,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null,
            null)
    {
    }
}

internal sealed record BackupRunExecutionPlanDto(
    Guid RunId,
    Guid RunBackupPolicyId,
    Guid RunBackupRepositoryId,
    string RunPolicyNameSnapshot,
    string RunSourceSnapshot,
    string RunRepositoryTypeSnapshot,
    string RunTrigger,
    Guid? RunTriggerSourceId,
    string RunStatus,
    string? RunResticSnapshotId,
    string? RunParentSnapshotId,
    string RunSnapshotAvailability,
    long? RunFilesProcessed,
    long? RunBytesProcessed,
    long? RunBytesAdded,
    string RunWarnings,
    DateTime RunQueuedAt,
    DateTime? RunStartedAt,
    DateTime? RunCompletedAt,
    int? RunExitCode,
    string? RunErrorCode,
    string? RunErrorMessage,
    Guid RunTriggeredByActorId,
    Guid PolicyId,
    string PolicyName,
    string PolicyNormalizedName,
    string? PolicyDescription,
    string PolicySource,
    Guid PolicyBackupRepositoryId,
    bool PolicyEnabled,
    string? PolicyCron,
    string? PolicyTimeZone,
    int PolicyKeepLastSuccessful,
    int PolicyTimeoutSeconds,
    bool PolicyAlertOnFailure,
    Guid PolicyRunAsActorId,
    string PolicyControlState,
    Guid? PolicyCurrentRunId,
    DateTime? PolicyLastScheduledRunAt,
    DateTime? PolicyFirstSuccessfulRunAt,
    Guid PolicyCreatedByActorId,
    DateTime PolicyCreatedAt,
    DateTime PolicyUpdatedAt,
    DateTime? PolicyArchivedAt,
    long PolicyRowVersion,
    Guid RepositoryId,
    string RepositoryName,
    string RepositoryNormalizedName,
    string? RepositoryDescription,
    string RepositoryType,
    string RepositorySpec,
    Guid RepositoryPasswordSecretId,
    string RepositoryStatus,
    DateTime? RepositoryLastPrunedAt,
    DateTime? RepositoryLastCheckedAt,
    Guid RepositoryCreatedByActorId,
    DateTime RepositoryCreatedAt,
    DateTime RepositoryUpdatedAt,
    DateTime? RepositoryArchivedAt,
    long RepositoryRowVersion)
{
    public BackupRunExecutionPlanDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            "{}",
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            null,
            null,
            string.Empty,
            null,
            null,
            null,
            "[]",
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            string.Empty,
            null,
            "{}",
            Guid.Empty,
            false,
            null,
            null,
            14,
            14_400,
            true,
            Guid.Empty,
            string.Empty,
            null,
            null,
            null,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue,
            null,
            0,
            Guid.Empty,
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            "{}",
            Guid.Empty,
            string.Empty,
            null,
            null,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue,
            null,
            0)
    {
    }
}

internal sealed record BackupRunLogDto(
    Guid Id,
    Guid BackupRunId,
    DateTime CreatedAt,
    string Stream,
    string Message)
{
    public BackupRunLogDto()
        : this(Guid.Empty, Guid.Empty, DateTime.MinValue, string.Empty, string.Empty)
    {
    }
}

internal sealed record BackupRestoreRunDto(
    Guid Id,
    Guid BackupRunId,
    Guid BackupRepositoryId,
    string Status,
    Guid TargetPlatformId,
    string TargetVolumeName,
    bool OverwriteExisting,
    bool TargetVolumeCreatedByCitadel,
    string AffectedContainers,
    string Warnings,
    DateTime QueuedAt,
    DateTime? StartedAt,
    DateTime? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid TriggeredByActorId)
{
    public BackupRestoreRunDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            Guid.Empty,
            string.Empty,
            false,
            false,
            "[]",
            "[]",
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty)
    {
    }
}

internal sealed record BackupRestoreRunExecutionPlanDto(
    Guid RestoreRunId,
    Guid RestoreBackupRunId,
    Guid RestoreBackupRepositoryId,
    string RestoreStatus,
    Guid RestoreTargetPlatformId,
    string RestoreTargetVolumeName,
    bool RestoreOverwriteExisting,
    bool RestoreTargetVolumeCreatedByCitadel,
    string RestoreAffectedContainers,
    string RestoreWarnings,
    DateTime RestoreQueuedAt,
    DateTime? RestoreStartedAt,
    DateTime? RestoreCompletedAt,
    int? RestoreExitCode,
    string? RestoreErrorCode,
    string? RestoreErrorMessage,
    Guid RestoreTriggeredByActorId,
    Guid RunId,
    Guid RunBackupPolicyId,
    Guid RunBackupRepositoryId,
    string RunPolicyNameSnapshot,
    string RunSourceSnapshot,
    string RunRepositoryTypeSnapshot,
    string RunTrigger,
    Guid? RunTriggerSourceId,
    string RunStatus,
    string? RunResticSnapshotId,
    string? RunParentSnapshotId,
    string RunSnapshotAvailability,
    long? RunFilesProcessed,
    long? RunBytesProcessed,
    long? RunBytesAdded,
    string RunWarnings,
    DateTime RunQueuedAt,
    DateTime? RunStartedAt,
    DateTime? RunCompletedAt,
    int? RunExitCode,
    string? RunErrorCode,
    string? RunErrorMessage,
    Guid RunTriggeredByActorId,
    Guid RepositoryId,
    string RepositoryName,
    string RepositoryNormalizedName,
    string? RepositoryDescription,
    string RepositoryType,
    string RepositorySpec,
    Guid RepositoryPasswordSecretId,
    string RepositoryStatus,
    DateTime? RepositoryLastPrunedAt,
    DateTime? RepositoryLastCheckedAt,
    Guid RepositoryCreatedByActorId,
    DateTime RepositoryCreatedAt,
    DateTime RepositoryUpdatedAt,
    DateTime? RepositoryArchivedAt,
    long RepositoryRowVersion)
{
    public BackupRestoreRunExecutionPlanDto()
        : this(
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            Guid.Empty,
            string.Empty,
            false,
            false,
            "[]",
            "[]",
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            "{}",
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            null,
            null,
            string.Empty,
            null,
            null,
            null,
            "[]",
            DateTime.MinValue,
            null,
            null,
            null,
            null,
            null,
            Guid.Empty,
            Guid.Empty,
            string.Empty,
            string.Empty,
            null,
            string.Empty,
            "{}",
            Guid.Empty,
            string.Empty,
            null,
            null,
            Guid.Empty,
            DateTime.MinValue,
            DateTime.MinValue,
            null,
            0)
    {
    }
}

internal sealed record BackupRestoreRunLogDto(
    Guid Id,
    Guid BackupRestoreRunId,
    DateTime CreatedAt,
    string Stream,
    string Message)
{
    public BackupRestoreRunLogDto()
        : this(Guid.Empty, Guid.Empty, DateTime.MinValue, string.Empty, string.Empty)
    {
    }
}

internal sealed record BackupRestoreRunLogWithRunStateDto(
    bool RunExists,
    Guid? LogId,
    Guid? LogBackupRestoreRunId,
    DateTime? LogCreatedAt,
    string? LogStream,
    string? LogMessage)
{
    public BackupRestoreRunLogWithRunStateDto()
        : this(false, null, null, null, null, null)
    {
    }
}
