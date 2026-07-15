using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Backups;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization.Metadata;

namespace Infrastructure.Persistence.Mappers;

internal static class BackupMappers
{
    internal static BackupRepository ToDomain(this BackupRepositoryDto dto)
        => BackupRepository.FromPersistence(
            dto.Id,
            dto.Name,
            dto.NormalizedName,
            dto.Description,
            DeserializeRepositorySpec(dto.Spec),
            dto.PasswordSecretId,
            Enum.Parse<BackupRepositoryStatus>(dto.Status),
            Enum.Parse<ResourceControlState>(dto.ControlState),
            dto.CurrentRunId,
            dto.ControlStartedAt,
            ToOffset(dto.LastPrunedAt),
            ToOffset(dto.LastCheckedAt),
            dto.CreatedByActorId,
            ToOffset(dto.CreatedAt),
            ToOffset(dto.UpdatedAt),
            ToOffset(dto.ArchivedAt),
            dto.RowVersion);

    internal static IEnumerable<BackupRepository> ToDomain(this IEnumerable<BackupRepositoryDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BackupRepositoryValidation ToDomain(this BackupRepositoryValidationDto dto)
        => BackupRepositoryValidation.FromPersistence(
            dto.Id,
            dto.BackupRepositoryId,
            Enum.Parse<BackupExecutionLocation>(dto.Location),
            dto.PlatformId,
            Enum.Parse<BackupRepositoryValidationStatus>(dto.Status),
            ToOffset(dto.LastValidatedAt),
            dto.LastErrorCode,
            dto.LastErrorMessage);

    internal static IEnumerable<BackupRepositoryValidation> ToDomain(this IEnumerable<BackupRepositoryValidationDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BackupPolicy ToDomain(this BackupPolicyDto dto)
    {
        var policy = BackupPolicy.FromPersistence(
            dto.Id,
            dto.Name,
            dto.NormalizedName,
            dto.Description,
            DeserializeSource(dto.Source),
            dto.BackupRepositoryId,
            dto.Enabled,
            dto.Cron,
            dto.TimeZone,
            DeserializeWebhook(dto.Webhook),
            dto.KeepLastSuccessful,
            dto.TimeoutSeconds,
            dto.AlertOnFailure,
            dto.RunAsActorId,
            Enum.Parse<ResourceControlState>(dto.ControlState),
            dto.CurrentRunId,
            dto.ControlStartedAt,
            ToOffset(dto.LastScheduledRunAt),
            ToOffset(dto.FirstSuccessfulRunAt),
            dto.CreatedByActorId,
            ToOffset(dto.CreatedAt),
            ToOffset(dto.UpdatedAt),
            ToOffset(dto.ArchivedAt),
            dto.RowVersion);

        policy.AssignTags(dto.TagsJson.ToTagSummaries());
        return policy;
    }

    internal static IEnumerable<BackupPolicy> ToDomain(this IEnumerable<BackupPolicyDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static ScheduledBackupPolicy ToDomain(this ScheduledBackupPolicyDto dto)
        => new(dto.Id, dto.Cron, dto.TimeZone);

    internal static IEnumerable<ScheduledBackupPolicy> ToDomain(this IEnumerable<ScheduledBackupPolicyDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static VolumeBackupCoverage ToDomain(this VolumeBackupCoverageDto dto)
        => new(
            new VolumeBackupCoverageKey(dto.PlatformId, dto.VolumeName),
            new BackupCoverageView(
                Enum.Parse<BackupCoverageStatus>(dto.Status),
                dto.PolicyCount,
                dto.LastRunId,
                dto.LastRunStatus is null ? null : Enum.Parse<BackupRunStatus>(dto.LastRunStatus),
                ToOffset(dto.LastRunAt),
                ToOffset(dto.LastSuccessfulRunAt),
                ToOffset(dto.NextRunAt)));

    internal static BackupRun ToDomain(this BackupRunDto dto)
        => BackupRun.FromPersistence(
            dto.Id,
            dto.BackupPolicyId,
            dto.BackupRepositoryId,
            dto.PolicyNameSnapshot,
            DeserializeSource(dto.SourceSnapshot),
            Enum.Parse<BackupRepositoryType>(dto.RepositoryTypeSnapshot),
            Enum.Parse<BackupRunTrigger>(dto.Trigger),
            dto.TriggerSourceId,
            Enum.Parse<BackupRunStatus>(dto.Status),
            dto.ResticSnapshotId,
            dto.ParentSnapshotId,
            Enum.Parse<BackupSnapshotAvailability>(dto.SnapshotAvailability),
            dto.FilesProcessed,
            dto.BytesProcessed,
            dto.BytesAdded,
            DeserializeWarnings(dto.Warnings),
            ToOffset(dto.QueuedAt),
            ToOffset(dto.StartedAt),
            ToOffset(dto.CompletedAt),
            dto.ExitCode,
            dto.ErrorCode,
            dto.ErrorMessage,
            dto.TriggeredByActorId);

    internal static IEnumerable<BackupRun> ToDomain(this IEnumerable<BackupRunDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BackupRunFinishOutcome ToDomain(this BackupRunFinishDto dto)
    {
        var status = dto.ResultStatus == 0
            ? BackupRunFinishResult.Completed
            : BackupRunFinishResult.AlreadyCancelled;

        if (status != BackupRunFinishResult.Completed || !dto.Id.HasValue)
            return new BackupRunFinishOutcome(status, null);

        var policy = new BackupPolicyDto(
            dto.Id.Value,
            dto.Name ?? string.Empty,
            dto.NormalizedName ?? string.Empty,
            dto.Description,
            dto.Source ?? "{}",
            dto.BackupRepositoryId.GetValueOrDefault(),
            dto.Enabled.GetValueOrDefault(),
            dto.Cron,
            dto.TimeZone,
            dto.Webhook,
            dto.KeepLastSuccessful.GetValueOrDefault(BackupPolicy.DefaultKeepLastSuccessful),
            dto.TimeoutSeconds.GetValueOrDefault(BackupPolicy.DefaultTimeoutSeconds),
            dto.AlertOnFailure.GetValueOrDefault(),
            dto.RunAsActorId.GetValueOrDefault(),
            dto.ControlState ?? ResourceControlState.Idle.ToString(),
            dto.CurrentRunId,
            dto.ControlStartedAt,
            dto.LastScheduledRunAt,
            dto.FirstSuccessfulRunAt,
            dto.CreatedByActorId.GetValueOrDefault(),
            dto.CreatedAt.GetValueOrDefault(),
            dto.UpdatedAt.GetValueOrDefault(),
            dto.ArchivedAt,
            dto.RowVersion.GetValueOrDefault(),
            dto.TagsJson);

        return new BackupRunFinishOutcome(status, policy.ToDomain());
    }

    internal static BackupRunItem ToDomain(this BackupRunItemDto dto)
        => BackupRunItem.FromPersistence(
            dto.Id,
            dto.BackupRunId,
            dto.PlatformId,
            dto.VolumeName,
            Enum.Parse<BackupRunItemStatus>(dto.Status),
            dto.ResticSnapshotId,
            dto.ParentSnapshotId,
            dto.FilesProcessed,
            dto.BytesProcessed,
            dto.BytesAdded,
            ToOffset(dto.StartedAt),
            ToOffset(dto.CompletedAt),
            dto.ExitCode,
            dto.ErrorCode,
            dto.ErrorMessage,
            ToOffset(dto.CreatedAt),
            ToOffset(dto.UpdatedAt));

    internal static IEnumerable<BackupRunItem> ToDomain(this IEnumerable<BackupRunItemDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BackupRunLogEntry ToDomain(this BackupRunLogDto dto)
        => new(
            dto.Id,
            dto.BackupRunId,
            ToOffset(dto.CreatedAt),
            dto.Stream,
            dto.Message);

    internal static IReadOnlyList<BackupRunLogEntry> ToDomain(this IEnumerable<BackupRunLogDto> dtos)
        => [.. dtos.Select(static dto => dto.ToDomain())];

    internal static BackupRun ToDomain(this BackupRunQueueDto dto)
        => BackupRun.FromPersistence(
            dto.Id ?? throw new InvalidOperationException("Queued backup run ID is missing."),
            dto.BackupPolicyId ?? throw new InvalidOperationException("Queued backup run policy ID is missing."),
            dto.BackupRepositoryId ?? throw new InvalidOperationException("Queued backup run repository ID is missing."),
            dto.PolicyNameSnapshot ?? throw new InvalidOperationException("Queued backup run policy name is missing."),
            DeserializeSource(dto.SourceSnapshot ?? throw new InvalidOperationException("Queued backup run source is missing.")),
            Enum.Parse<BackupRepositoryType>(dto.RepositoryTypeSnapshot ?? throw new InvalidOperationException("Queued backup run repository type is missing.")),
            Enum.Parse<BackupRunTrigger>(dto.Trigger ?? throw new InvalidOperationException("Queued backup run trigger is missing.")),
            dto.TriggerSourceId,
            Enum.Parse<BackupRunStatus>(dto.Status ?? throw new InvalidOperationException("Queued backup run status is missing.")),
            dto.ResticSnapshotId,
            dto.ParentSnapshotId,
            Enum.Parse<BackupSnapshotAvailability>(dto.SnapshotAvailability ?? throw new InvalidOperationException("Queued backup run snapshot availability is missing.")),
            dto.FilesProcessed,
            dto.BytesProcessed,
            dto.BytesAdded,
            DeserializeWarnings(dto.Warnings),
            ToOffset(dto.QueuedAt ?? throw new InvalidOperationException("Queued backup run timestamp is missing.")),
            ToOffset(dto.StartedAt),
            ToOffset(dto.CompletedAt),
            dto.ExitCode,
            dto.ErrorCode,
            dto.ErrorMessage,
            dto.TriggeredByActorId ?? throw new InvalidOperationException("Queued backup run actor is missing."));

    internal static BackupRunExecutionPlan ToDomain(this BackupRunExecutionPlanDto dto)
        => new(
            new BackupRunDto(
                dto.RunId,
                dto.RunBackupPolicyId,
                dto.RunBackupRepositoryId,
                dto.RunPolicyNameSnapshot,
                dto.RunSourceSnapshot,
                dto.RunRepositoryTypeSnapshot,
                dto.RunTrigger,
                dto.RunTriggerSourceId,
                dto.RunStatus,
                dto.RunResticSnapshotId,
                dto.RunParentSnapshotId,
                dto.RunSnapshotAvailability,
                dto.RunFilesProcessed,
                dto.RunBytesProcessed,
                dto.RunBytesAdded,
                dto.RunWarnings,
                dto.RunQueuedAt,
                dto.RunStartedAt,
                dto.RunCompletedAt,
                dto.RunExitCode,
                dto.RunErrorCode,
                dto.RunErrorMessage,
                dto.RunTriggeredByActorId).ToDomain(),
            new BackupPolicyDto(
                dto.PolicyId,
                dto.PolicyName,
                dto.PolicyNormalizedName,
                dto.PolicyDescription,
                dto.PolicySource,
                dto.PolicyBackupRepositoryId,
                dto.PolicyEnabled,
                dto.PolicyCron,
                dto.PolicyTimeZone,
                dto.PolicyWebhook,
                dto.PolicyKeepLastSuccessful,
                dto.PolicyTimeoutSeconds,
                dto.PolicyAlertOnFailure,
                dto.PolicyRunAsActorId,
                dto.PolicyControlState,
                dto.PolicyCurrentRunId,
                dto.PolicyControlStartedAt,
                dto.PolicyLastScheduledRunAt,
                dto.PolicyFirstSuccessfulRunAt,
                dto.PolicyCreatedByActorId,
                dto.PolicyCreatedAt,
                dto.PolicyUpdatedAt,
                dto.PolicyArchivedAt,
                dto.PolicyRowVersion).ToDomain(),
            new BackupRepositoryDto(
                dto.RepositoryId,
                dto.RepositoryName,
                dto.RepositoryNormalizedName,
                dto.RepositoryDescription,
                dto.RepositoryType,
                dto.RepositorySpec,
                dto.RepositoryPasswordSecretId,
                dto.RepositoryStatus,
                dto.RepositoryControlState,
                dto.RepositoryCurrentRunId,
                dto.RepositoryControlStartedAt,
                dto.RepositoryLastPrunedAt,
                dto.RepositoryLastCheckedAt,
                dto.RepositoryCreatedByActorId,
                dto.RepositoryCreatedAt,
                dto.RepositoryUpdatedAt,
                dto.RepositoryArchivedAt,
                dto.RepositoryRowVersion).ToDomain());

    internal static BackupRestoreRun ToDomain(this BackupRestoreRunDto dto)
        => BackupRestoreRun.FromPersistence(
            dto.Id,
            dto.BackupRunId,
            dto.BackupRepositoryId,
            Enum.Parse<BackupRestoreStatus>(dto.Status),
            dto.TargetPlatformId,
            dto.TargetVolumeName,
            dto.OverwriteExisting,
            dto.TargetVolumeCreatedByCitadel,
            DeserializeAffectedContainers(dto.AffectedContainers),
            DeserializeWarnings(dto.Warnings),
            ToOffset(dto.QueuedAt),
            ToOffset(dto.StartedAt),
            ToOffset(dto.CompletedAt),
            dto.ExitCode,
            dto.ErrorCode,
            dto.ErrorMessage,
            dto.TriggeredByActorId);

    internal static IEnumerable<BackupRestoreRun> ToDomain(this IEnumerable<BackupRestoreRunDto> dtos)
        => dtos.Select(static dto => dto.ToDomain());

    internal static BackupRestoreRunWithPolicy ToDomain(this BackupRestoreRunWithPolicyDto dto)
        => new(
            new BackupRestoreRunDto(
                dto.Id,
                dto.BackupRunId,
                dto.BackupRepositoryId,
                dto.Status,
                dto.TargetPlatformId,
                dto.TargetVolumeName,
                dto.OverwriteExisting,
                dto.TargetVolumeCreatedByCitadel,
                dto.AffectedContainers,
                dto.Warnings,
                dto.QueuedAt,
                dto.StartedAt,
                dto.CompletedAt,
                dto.ExitCode,
                dto.ErrorCode,
                dto.ErrorMessage,
                dto.TriggeredByActorId).ToDomain(),
            dto.BackupPolicyId);

    internal static BackupRestoreRunExecutionPlan ToDomain(this BackupRestoreRunExecutionPlanDto dto)
        => new(
            new BackupRestoreRunDto(
                dto.RestoreRunId,
                dto.RestoreBackupRunId,
                dto.RestoreBackupRepositoryId,
                dto.RestoreStatus,
                dto.RestoreTargetPlatformId,
                dto.RestoreTargetVolumeName,
                dto.RestoreOverwriteExisting,
                dto.RestoreTargetVolumeCreatedByCitadel,
                dto.RestoreAffectedContainers,
                dto.RestoreWarnings,
                dto.RestoreQueuedAt,
                dto.RestoreStartedAt,
                dto.RestoreCompletedAt,
                dto.RestoreExitCode,
                dto.RestoreErrorCode,
                dto.RestoreErrorMessage,
                dto.RestoreTriggeredByActorId).ToDomain(),
            new BackupRunDto(
                dto.RunId,
                dto.RunBackupPolicyId,
                dto.RunBackupRepositoryId,
                dto.RunPolicyNameSnapshot,
                dto.RunSourceSnapshot,
                dto.RunRepositoryTypeSnapshot,
                dto.RunTrigger,
                dto.RunTriggerSourceId,
                dto.RunStatus,
                dto.RunResticSnapshotId,
                dto.RunParentSnapshotId,
                dto.RunSnapshotAvailability,
                dto.RunFilesProcessed,
                dto.RunBytesProcessed,
                dto.RunBytesAdded,
                dto.RunWarnings,
                dto.RunQueuedAt,
                dto.RunStartedAt,
                dto.RunCompletedAt,
                dto.RunExitCode,
                dto.RunErrorCode,
                dto.RunErrorMessage,
                dto.RunTriggeredByActorId).ToDomain(),
            new BackupRepositoryDto(
                dto.RepositoryId,
                dto.RepositoryName,
                dto.RepositoryNormalizedName,
                dto.RepositoryDescription,
                dto.RepositoryType,
                dto.RepositorySpec,
                dto.RepositoryPasswordSecretId,
                dto.RepositoryStatus,
                dto.RepositoryControlState,
                dto.RepositoryCurrentRunId,
                dto.RepositoryControlStartedAt,
                dto.RepositoryLastPrunedAt,
                dto.RepositoryLastCheckedAt,
                dto.RepositoryCreatedByActorId,
                dto.RepositoryCreatedAt,
                dto.RepositoryUpdatedAt,
                dto.RepositoryArchivedAt,
                dto.RepositoryRowVersion).ToDomain());

    internal static BackupRestoreRunLogEntry ToDomain(this BackupRestoreRunLogDto dto)
        => new(
            dto.Id,
            dto.BackupRestoreRunId,
            ToOffset(dto.CreatedAt),
            dto.Stream,
            dto.Message);

    internal static IReadOnlyList<BackupRestoreRunLogEntry> ToDomain(this IEnumerable<BackupRestoreRunLogDto> dtos)
        => [.. dtos.Select(static dto => dto.ToDomain())];

    internal static string SerializeSource(BackupSourceSpec source)
        => source switch
        {
            DockerVolumeBackupSource volume => SerializeWithType("DockerVolume", volume, BackupJsonContext.Default.DockerVolumeBackupSource),
            CitadelSystemBackupSource system => SerializeWithType("CitadelSystem", system, BackupJsonContext.Default.CitadelSystemBackupSource),
            StackBackupSource stack => SerializeWithType("Stack", stack, BackupJsonContext.Default.StackBackupSource),
            DeploymentBackupSource deployment => SerializeWithType("Deployment", deployment, BackupJsonContext.Default.DeploymentBackupSource),
            _ => throw new NotSupportedException($"Backup source type '{source.GetType().Name}' is not supported.")
        };

    internal static string SerializeRepositorySpec(BackupRepositorySpec spec)
        => spec switch
        {
            FileSystemBackupRepositorySpec fileSystem => SerializeWithType("FileSystem", fileSystem, BackupJsonContext.Default.FileSystemBackupRepositorySpec),
            S3CompatibleBackupRepositorySpec s3 => SerializeWithType("S3Compatible", s3, BackupJsonContext.Default.S3CompatibleBackupRepositorySpec),
            _ => throw new NotSupportedException($"Backup repository spec type '{spec.GetType().Name}' is not supported.")
        };

    internal static string? SerializeWebhook(BackupWebhookConfig? webhook)
        => webhook is null
            ? null
            : JsonSerializer.Serialize(webhook, BackupJsonContext.Default.BackupWebhookConfig);

    internal static string SerializeWarnings(IReadOnlyList<BackupRunWarning> warnings)
        => JsonSerializer.Serialize(warnings, BackupJsonContext.Default.IReadOnlyListBackupRunWarning);

    internal static string SerializeAffectedContainers(IReadOnlyList<BackupAffectedContainer> containers)
        => JsonSerializer.Serialize(containers, BackupJsonContext.Default.IReadOnlyListBackupAffectedContainer);

    internal static DateTime ToUtcDateTime(DateTimeOffset value)
        => value.ToUniversalTime().UtcDateTime;

    internal static DateTime? ToUtcDateTime(DateTimeOffset? value)
        => value?.ToUniversalTime().UtcDateTime;

    private static BackupSourceSpec DeserializeSource(string json)
    {
        var type = ReadDiscriminator(json, "Backup source JSON is missing a type discriminator.");
        return type switch
        {
            "DockerVolume" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.DockerVolumeBackupSource)
                              ?? throw new InvalidOperationException("Backup source JSON is invalid."),
            "CitadelSystem" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.CitadelSystemBackupSource)
                               ?? throw new InvalidOperationException("Backup source JSON is invalid."),
            "Stack" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.StackBackupSource)
                       ?? throw new InvalidOperationException("Backup source JSON is invalid."),
            "Deployment" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.DeploymentBackupSource)
                            ?? throw new InvalidOperationException("Backup source JSON is invalid."),
            _ => throw new NotSupportedException($"Backup source type '{type}' is not supported.")
        };
    }

    private static BackupRepositorySpec DeserializeRepositorySpec(string json)
    {
        var type = ReadDiscriminator(json, "Backup repository JSON is missing a type discriminator.");
        return type switch
        {
            "FileSystem" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.FileSystemBackupRepositorySpec)
                            ?? throw new InvalidOperationException("Backup repository JSON is invalid."),
            "S3Compatible" => JsonSerializer.Deserialize(json, BackupJsonContext.Default.S3CompatibleBackupRepositorySpec)
                              ?? throw new InvalidOperationException("Backup repository JSON is invalid."),
            _ => throw new NotSupportedException($"Backup repository type '{type}' is not supported.")
        };
    }

    private static BackupWebhookConfig? DeserializeWebhook(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? null
            : JsonSerializer.Deserialize(json, BackupJsonContext.Default.BackupWebhookConfig);

    private static IReadOnlyList<BackupRunWarning> DeserializeWarnings(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? []
            : JsonSerializer.Deserialize(json, BackupJsonContext.Default.IReadOnlyListBackupRunWarning) ?? [];

    private static IReadOnlyList<BackupAffectedContainer> DeserializeAffectedContainers(string? json)
        => string.IsNullOrWhiteSpace(json)
            ? []
            : JsonSerializer.Deserialize(json, BackupJsonContext.Default.IReadOnlyListBackupAffectedContainer) ?? [];

    internal static DateTimeOffset ToOffset(DateTime value)
        => new(DateTime.SpecifyKind(value, DateTimeKind.Utc));

    internal static DateTimeOffset? ToOffset(DateTime? value)
        => value.HasValue ? ToOffset(value.Value) : null;

    private static string SerializeWithType<T>(string type, T value, JsonTypeInfo<T> jsonTypeInfo)
    {
        var payload = JsonSerializer.SerializeToNode(value, jsonTypeInfo)?.AsObject()
                      ?? throw new InvalidOperationException("Backup JSON payload is invalid.");
        var result = new JsonObject { ["$type"] = type };

        foreach (var property in payload.ToArray())
        {
            payload.Remove(property.Key);
            result[property.Key] = property.Value;
        }

        return result.ToJsonString();
    }

    private static string ReadDiscriminator(string json, string missingMessage)
    {
        using var document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty("$type", out var typeProperty))
            throw new NotSupportedException(missingMessage);

        return typeProperty.GetString() ?? throw new NotSupportedException(missingMessage);
    }
}
