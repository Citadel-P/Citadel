using Application.Features.Backups.Models;
using Application.Permissions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints.Resources.Backups;

public sealed record BackupRepositoryView(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    BackupRepositoryType Type,
    BackupRepositorySpec Spec,
    Guid PasswordSecretId,
    BackupRepositoryStatus Status,
    ResourceControlState ControlState,
    Guid? CurrentRunId,
    long? ControlStartedAt,
    DateTimeOffset? LastPrunedAt,
    DateTimeOffset? LastCheckedAt,
    Guid CreatedByActorId,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    DateTimeOffset? ArchivedAt,
    long RowVersion,
    ResourceCapabilities? Capabilities = null)
{
    internal static BackupRepositoryView Map(BackupRepositoryResult result)
        => Map(result.Repository);

    internal static async Task<BackupRepositoryView> Map(BackupRepositoryResult result, IPermissionEvaluator permissionEvaluator)
        => await Map(result.Repository, permissionEvaluator);

    internal static BackupRepositoryView Map(BackupRepository repository)
        => new(
            repository.Id,
            repository.Name,
            repository.NormalizedName,
            repository.Description,
            repository.Type,
            repository.Spec,
            repository.PasswordSecretId,
            repository.Status,
            repository.ControlState,
            repository.CurrentRunId,
            repository.ControlStartedAt,
            repository.LastPrunedAt,
            repository.LastCheckedAt,
            repository.CreatedByActorId,
            repository.CreatedAt,
            repository.UpdatedAt,
            repository.ArchivedAt,
            repository.RowVersion);

    internal static async Task<BackupRepositoryView> Map(BackupRepository repository, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(repository.Id, ResourceType.BackupRepository);
        return Map(repository) with
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }
}

public sealed record BackupRepositoriesView(IReadOnlyList<BackupRepositoryView> Repositories, ResourceCapabilities Capabilities)
{
    internal static async Task<BackupRepositoriesView> Map(BackupRepositoryListResult result, IPermissionEvaluator permissionEvaluator)
    {
        var repositories = result.Repositories;
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.BackupRepository);

        if (repositories.Count == 0)
            return new BackupRepositoriesView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = repositories.Select(static x => x.Id).ToArray();
        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.BackupRepository);
        var views = new BackupRepositoryView[repositories.Count];

        for (var i = 0; i < repositories.Count; i++)
        {
            var repository = repositories[i];
            perms.TryGetValue(repository.Id, out var meta);
            views[i] = BackupRepositoryView.Map(repository) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new BackupRepositoriesView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

public sealed record BackupRepositoryValidationView(
    Guid Id,
    Guid BackupRepositoryId,
    BackupExecutionLocation Location,
    Guid? PlatformId,
    BackupRepositoryValidationStatus Status,
    DateTimeOffset LastValidatedAt,
    string? LastErrorCode,
    string? LastErrorMessage)
{
    internal static BackupRepositoryValidationView Map(BackupRepositoryValidation validation)
        => new(
            validation.Id,
            validation.BackupRepositoryId,
            validation.Location,
            validation.PlatformId,
            validation.Status,
            validation.LastValidatedAt,
            validation.LastErrorCode,
            validation.LastErrorMessage);
}

public sealed record BackupPolicyView(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    BackupSourceSpec Source,
    Guid BackupRepositoryId,
    bool Enabled,
    string? Cron,
    string? TimeZone,
    BackupWebhookConfig? Webhook,
    int KeepLastSuccessful,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId,
    ResourceControlState ControlState,
    Guid? CurrentRunId,
    DateTimeOffset? LastScheduledRunAt,
    DateTimeOffset? FirstSuccessfulRunAt,
    Guid CreatedByActorId,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    DateTimeOffset? ArchivedAt,
    long RowVersion,
    BackupRunView? LatestRun,
    IReadOnlyList<TagSummaryView> Tags,
    ResourceCapabilities? Capabilities = null)
{
    internal static BackupPolicyView Map(BackupPolicyResult result)
        => Map(result.Policy, result.LatestRun);

    internal static async Task<BackupPolicyView> Map(BackupPolicyResult result, IPermissionEvaluator permissionEvaluator)
        => await Map(result.Policy, permissionEvaluator, result.LatestRun);

    internal static BackupPolicyView Map(BackupPolicy policy)
        => Map(policy, latestRun: null);

    internal static BackupPolicyView Map(BackupPolicy policy, BackupRun? latestRun)
        => new(
            policy.Id,
            policy.Name,
            policy.NormalizedName,
            policy.Description,
            policy.Source,
            policy.BackupRepositoryId,
            policy.Enabled,
            policy.Cron,
            policy.TimeZone,
            policy.Webhook,
            policy.KeepLastSuccessful,
            policy.TimeoutSeconds,
            policy.AlertOnFailure,
            policy.RunAsActorId,
            policy.ControlState,
            policy.CurrentRunId,
            policy.LastScheduledRunAt,
            policy.FirstSuccessfulRunAt,
            policy.CreatedByActorId,
            policy.CreatedAt,
            policy.UpdatedAt,
            policy.ArchivedAt,
            policy.RowVersion,
            latestRun is null ? null : BackupRunView.Map(latestRun),
            [.. policy.Tags.Select(TagSummaryView.Map)]);

    internal static async Task<BackupPolicyView> Map(
        BackupPolicy policy,
        IPermissionEvaluator permissionEvaluator,
        BackupRun? latestRun = null)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(policy.Id, ResourceType.BackupPolicy);
        return Map(policy, latestRun) with
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }
}

public sealed record BackupPoliciesView(IReadOnlyList<BackupPolicyView> Policies, ResourceCapabilities Capabilities)
{
    internal static async Task<BackupPoliciesView> Map(BackupPolicyListResult result, IPermissionEvaluator permissionEvaluator)
    {
        var policies = result.Policies;
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.BackupPolicy);

        if (policies.Count == 0)
            return new BackupPoliciesView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = policies.Select(static x => x.Id).ToArray();
        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.BackupPolicy);
        var views = new BackupPolicyView[policies.Count];

        for (var i = 0; i < policies.Count; i++)
        {
            var policy = policies[i];
            perms.TryGetValue(policy.Id, out var meta);
            views[i] = BackupPolicyView.Map(policy, result.LatestRuns.GetValueOrDefault(policy.Id)) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new BackupPoliciesView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

public sealed record PlatformBackupSummaryView(
    Guid PlatformId,
    int PolicyCount,
    int EnabledPolicyCount,
    int DockerVolumePolicyCount,
    int StackPolicyCount,
    int DeploymentPolicyCount,
    int AttentionPolicyCount,
    BackupRunStatus? LastRunStatus,
    DateTimeOffset? LastRunAt)
{
    internal static PlatformBackupSummaryView Map(PlatformBackupSummary summary)
        => new(
            summary.PlatformId,
            summary.PolicyCount,
            summary.EnabledPolicyCount,
            summary.DockerVolumePolicyCount,
            summary.StackPolicyCount,
            summary.DeploymentPolicyCount,
            summary.AttentionPolicyCount,
            summary.LastRunStatus,
            summary.LastRunAt);
}

public sealed record PlatformBackupSummariesView(IReadOnlyList<PlatformBackupSummaryView> Platforms)
{
    internal static PlatformBackupSummariesView Map(IReadOnlyList<PlatformBackupSummary> summaries)
        => new([.. summaries.Select(PlatformBackupSummaryView.Map)]);
}

public sealed record BackupRunView(
    Guid Id,
    Guid BackupPolicyId,
    Guid BackupRepositoryId,
    string PolicyNameSnapshot,
    BackupSourceSpec SourceSnapshot,
    BackupRepositoryType RepositoryTypeSnapshot,
    BackupRunTrigger Trigger,
    Guid? TriggerSourceId,
    BackupRunStatus Status,
    string? ResticSnapshotId,
    string? ParentSnapshotId,
    BackupSnapshotAvailability SnapshotAvailability,
    long? FilesProcessed,
    long? BytesProcessed,
    long? BytesAdded,
    IReadOnlyList<BackupRunWarning> Warnings,
    DateTimeOffset QueuedAt,
    DateTimeOffset? StartedAt,
    DateTimeOffset? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid TriggeredByActorId,
    IReadOnlyList<BackupRunItemView> Items)
{
    internal static BackupRunView Map(BackupRunResult result)
        => Map(result.Run);

    internal static BackupRunView Map(BackupRun run)
        => new(
            run.Id,
            run.BackupPolicyId,
            run.BackupRepositoryId,
            run.PolicyNameSnapshot,
            run.SourceSnapshot,
            run.RepositoryTypeSnapshot,
            run.Trigger,
            run.TriggerSourceId,
            run.Status,
            run.ResticSnapshotId,
            run.ParentSnapshotId,
            run.SnapshotAvailability,
            run.FilesProcessed,
            run.BytesProcessed,
            run.BytesAdded,
            run.Warnings,
            run.QueuedAt,
            run.StartedAt,
            run.CompletedAt,
            run.ExitCode,
            run.ErrorCode,
            run.ErrorMessage,
            run.TriggeredByActorId,
            [.. run.Items.Select(BackupRunItemView.Map)]);
}

public sealed record BackupRunItemView(
    Guid Id,
    Guid BackupRunId,
    Guid PlatformId,
    string VolumeName,
    BackupRunItemStatus Status,
    string? ResticSnapshotId,
    string? ParentSnapshotId,
    long? FilesProcessed,
    long? BytesProcessed,
    long? BytesAdded,
    DateTimeOffset? StartedAt,
    DateTimeOffset? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage)
{
    internal static BackupRunItemView Map(BackupRunItem item)
        => new(
            item.Id,
            item.BackupRunId,
            item.PlatformId,
            item.VolumeName,
            item.Status,
            item.ResticSnapshotId,
            item.ParentSnapshotId,
            item.FilesProcessed,
            item.BytesProcessed,
            item.BytesAdded,
            item.StartedAt,
            item.CompletedAt,
            item.ExitCode,
            item.ErrorCode,
            item.ErrorMessage);
}

public sealed record BackupRunsView(IReadOnlyList<BackupRunView> Runs)
{
    internal static BackupRunsView Map(BackupRunListResult result)
        => new([.. result.Runs.Select(BackupRunView.Map)]);
}

public sealed record BackupRestoreRunView(
    Guid Id,
    Guid BackupRunId,
    Guid BackupRepositoryId,
    BackupRestoreStatus Status,
    Guid TargetPlatformId,
    string TargetVolumeName,
    bool OverwriteExisting,
    bool TargetVolumeCreatedByCitadel,
    IReadOnlyList<BackupAffectedContainer> AffectedContainers,
    IReadOnlyList<BackupRunWarning> Warnings,
    DateTimeOffset QueuedAt,
    DateTimeOffset? StartedAt,
    DateTimeOffset? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid TriggeredByActorId)
{
    internal static BackupRestoreRunView Map(BackupRestoreRunResult result)
        => Map(result.Run);

    internal static BackupRestoreRunView Map(BackupRestoreRun run)
        => new(
            run.Id,
            run.BackupRunId,
            run.BackupRepositoryId,
            run.Status,
            run.TargetPlatformId,
            run.TargetVolumeName,
            run.OverwriteExisting,
            run.TargetVolumeCreatedByCitadel,
            run.AffectedContainers,
            run.Warnings,
            run.QueuedAt,
            run.StartedAt,
            run.CompletedAt,
            run.ExitCode,
            run.ErrorCode,
            run.ErrorMessage,
            run.TriggeredByActorId);
}

public sealed record BackupRestoreRunsView(IReadOnlyList<BackupRestoreRunView> Runs)
{
    internal static BackupRestoreRunsView Map(BackupRestoreRunListResult result)
        => new([.. result.Runs.Select(BackupRestoreRunView.Map)]);
}

public sealed record BackupLogsView(Guid RunId, string Logs)
{
    internal static BackupLogsView Map(BackupRunLogResult result)
        => Map(result.RunId, result.Logs);

    internal static BackupLogsView Map(BackupRestoreRunLogResult result)
        => Map(result.RunId, result.Logs);

    private static BackupLogsView Map(Guid runId, IEnumerable<BackupRunLogEntry> logs)
        => new(
            runId,
            string.Join(
                Environment.NewLine,
                logs.Select(static log => log.Stream == "stderr" ? $"[stderr] {log.Message}" : log.Message)));

    private static BackupLogsView Map(Guid runId, IEnumerable<BackupRestoreRunLogEntry> logs)
        => new(
            runId,
            string.Join(
                Environment.NewLine,
                logs.Select(static log => log.Stream == "stderr" ? $"[stderr] {log.Message}" : log.Message)));
}

public sealed record BackupEventsView(Guid RunId, IReadOnlyList<string> Events);

public sealed record BackupRepositoryInput(
    string Name,
    string? Description,
    BackupRepositorySpec Spec,
    Guid PasswordSecretId)
{
    internal BackupRepositoryInputModel ToModel()
        => new(Name, Description, Spec, PasswordSecretId);
}

public sealed record UpdateBackupRepositoryInput(
    string? Description = null,
    BackupRepositorySpec? Spec = null)
{
    internal UpdateBackupRepositoryInputModel ToModel()
        => new(Description, Spec);
}

public sealed record ValidateBackupRepositoryInput(
    BackupExecutionLocation Location,
    Guid? PlatformId)
{
    internal ValidateBackupRepositoryInputModel ToModel()
        => new(Location, PlatformId);
}

public sealed record BackupPolicyInput(
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
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal BackupPolicyInputModel ToModel()
        => new(Name, Description, Source, BackupRepositoryId, Enabled, Cron, TimeZone, Webhook, KeepLastSuccessful, TimeoutSeconds, AlertOnFailure, RunAsActorId, TagIds);
}

public sealed record UpdateBackupPolicyInput(
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
    Guid? RunAsActorId = null)
{
    internal UpdateBackupPolicyInputModel ToModel()
        => new(Description, Source, BackupRepositoryId, Enabled, Cron, TimeZone, Webhook, KeepLastSuccessful, TimeoutSeconds, AlertOnFailure, RunAsActorId);
}

public sealed record QueueBackupRunInput(BackupRunTrigger Trigger = BackupRunTrigger.Manual, Guid? TriggerSourceId = null)
{
    internal QueueBackupRunInputModel ToModel()
        => new(Trigger, TriggerSourceId);
}

public sealed record RestoreVolumeInput(Guid TargetPlatformId, string TargetVolumeName, bool OverwriteExisting)
{
    internal RestoreVolumeInputModel ToModel()
        => new(TargetPlatformId, TargetVolumeName, OverwriteExisting);
}
