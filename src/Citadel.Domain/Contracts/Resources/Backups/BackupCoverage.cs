using Domain.Entities.Backups;

namespace Domain.Contracts.Resources.Backups;

public sealed record VolumeBackupCoverageKey(Guid PlatformId, string VolumeName, string? DockerNodeId = null);

public sealed record BackupCoverageView(
    BackupCoverageStatus Status,
    int PolicyCount,
    Guid? LastRunId,
    BackupRunStatus? LastRunStatus,
    DateTimeOffset? LastRunAt,
    DateTimeOffset? LastSuccessfulRunAt,
    DateTimeOffset? NextRunAt);

public sealed record VolumeBackupCoverage(VolumeBackupCoverageKey Resource, BackupCoverageView Coverage);

public sealed record PlatformBackupSummary(
    Guid PlatformId,
    int PolicyCount,
    int EnabledPolicyCount,
    int DockerVolumePolicyCount,
    int StackPolicyCount,
    int DeploymentPolicyCount,
    int SwarmServicePolicyCount,
    int AttentionPolicyCount,
    BackupRunStatus? LastRunStatus,
    DateTimeOffset? LastRunAt);
