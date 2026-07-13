using Domain.Entities.Backups;

namespace Domain.Contracts.Resources.Backups;

public sealed record VolumeBackupCoverageKey(Guid PlatformId, string VolumeName);

public sealed record BackupCoverageView(
    BackupCoverageStatus Status,
    int PolicyCount,
    Guid? LastRunId,
    BackupRunStatus? LastRunStatus,
    DateTimeOffset? LastRunAt,
    DateTimeOffset? LastSuccessfulRunAt,
    DateTimeOffset? NextRunAt);

public sealed record VolumeBackupCoverage(VolumeBackupCoverageKey Resource, BackupCoverageView Coverage);
