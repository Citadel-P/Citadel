using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;

namespace Application.Features.Backups.Models;

public sealed record BackupRepositoryInputModel(
    string Name,
    string? Description,
    BackupRepositorySpec Spec,
    Guid PasswordSecretId);

public sealed record UpdateBackupRepositoryInputModel(
    string? Description = null,
    BackupRepositorySpec? Spec = null);

public sealed record BackupRepositoryResult(BackupRepository Repository);

public sealed record BackupRepositoryListResult(IReadOnlyList<BackupRepository> Repositories);

public sealed record ValidateBackupRepositoryInputModel(
    BackupExecutionLocation Location,
    Guid? PlatformId);

public sealed record BackupPolicyInputModel(
    string Name,
    string? Description,
    BackupSourceSpec Source,
    Guid BackupRepositoryId,
    bool Enabled = true,
    string? Cron = null,
    string? TimeZone = null,
    BackupWebhookConfig? Webhook = null,
    int? KeepLastSuccessful = null,
    int? TimeoutSeconds = null,
    bool AlertOnFailure = true,
    Guid? RunAsActorId = null,
    IReadOnlyCollection<Guid>? TagIds = null);

public sealed record UpdateBackupPolicyInputModel(
    string? Description = null,
    BackupSourceSpec? Source = null,
    Guid? BackupRepositoryId = null,
    bool? Enabled = null,
    string? Cron = null,
    string? TimeZone = null,
    BackupWebhookConfig? Webhook = null,
    int? KeepLastSuccessful = null,
    int? TimeoutSeconds = null,
    bool? AlertOnFailure = null,
    Guid? RunAsActorId = null);

public sealed record BackupPolicyResult(BackupPolicy Policy);

public sealed record BackupPolicyListResult(IReadOnlyList<BackupPolicy> Policies);

public sealed record QueueBackupRunInputModel(
    BackupRunTrigger Trigger = BackupRunTrigger.Manual,
    Guid? TriggerSourceId = null);

public sealed record BackupRunResult(BackupRun Run);

public sealed record BackupRunListResult(IReadOnlyList<BackupRun> Runs);

public sealed record BackupRunLogResult(Guid RunId, IReadOnlyList<BackupRunLogEntry> Logs);

public sealed record BackupRestoreRunLogResult(Guid RunId, IReadOnlyList<BackupRestoreRunLogEntry> Logs);

public sealed record RestoreVolumeInputModel(
    Guid TargetPlatformId,
    string TargetVolumeName,
    bool OverwriteExisting);

public sealed record BackupRestoreRunResult(BackupRestoreRun Run, Guid BackupPolicyId);

public sealed record BackupRestoreRunListResult(IReadOnlyList<BackupRestoreRun> Runs);

public sealed record BackupCoverageResourceKey(
    Guid? ResourceId = null,
    Guid? PlatformId = null,
    string? Name = null);

public sealed record BackupCoverageItem(
    BackupCoverageResourceKey Resource,
    BackupCoverageView Coverage);

public sealed record BackupCoverageResult(IReadOnlyList<BackupCoverageItem> Items);

public sealed record StackBackupSourcePreviewResult(
    Guid StackId,
    string StackName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<StackBackupVolumePreviewItem> Volumes,
    IReadOnlyList<string> Warnings);

public sealed record StackBackupVolumePreviewItem(
    string Name,
    StackVolumeKind Kind,
    bool IsExternal,
    bool IsShared,
    bool HasBackupCoverage);

public sealed record DeploymentBackupSourcePreviewResult(
    Guid DeploymentId,
    string DeploymentName,
    Guid PlatformId,
    string PlatformName,
    PlatformStatus PlatformStatus,
    IReadOnlyList<StackBackupVolumePreviewItem> Volumes,
    IReadOnlyList<string> Warnings);
