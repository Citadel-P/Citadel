using Domain.Entities;
using Domain.Entities.Tags;
using Domain.Contracts.Resources;

namespace Domain.Entities.Backups;

public sealed class BackupPolicy(
    string name,
    string? description,
    BackupSourceSpec source,
    Guid backupRepositoryId,
    bool enabled,
    string? cron,
    string? timeZone,
    BackupWebhookConfig? webhook,
    int keepLastSuccessful,
    int timeoutSeconds,
    bool alertOnFailure,
    Guid runAsActorId,
    Guid createdByActorId,
    ResourceControlState controlState = ResourceControlState.Idle,
    Guid? currentRunId = null,
    long? controlStartedAt = null,
    DateTimeOffset? lastScheduledRunAt = null,
    DateTimeOffset? firstSuccessfulRunAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null) : IAuditedEntity, IReconcilableResource
{
    public const int DefaultKeepLastSuccessful = 14;
    public const int DefaultTimeoutSeconds = 14_400;

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = BackupRepository.NormalizeName(name);
    public string NormalizedName { get; private set; } = BackupRepository.ToNormalizedName(name);
    public string? Description { get; private set; } = BackupRepository.NormalizeOptional(description);
    public BackupSourceSpec Source { get; private set; } = NormalizeSource(source);
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public bool Enabled { get; private set; } = enabled;
    public string? Cron { get; private set; } = BackupRepository.NormalizeOptional(cron);
    public string? TimeZone { get; private set; } = BackupRepository.NormalizeOptional(timeZone);
    public BackupWebhookConfig? Webhook { get; private set; } = NormalizeWebhook(webhook);
    public bool WebhookEnabled => Webhook?.Enabled == true;
    public int KeepLastSuccessful { get; private set; } = keepLastSuccessful;
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public bool AlertOnFailure { get; private set; } = alertOnFailure;
    public Guid RunAsActorId { get; private set; } = runAsActorId;
    public ResourceControlState ControlState { get; private set; } = controlState;
    public Guid? CurrentRunId { get; private set; } = currentRunId;
    public long? ControlStartedAt { get; private set; } = controlStartedAt;
    public DateTimeOffset? LastScheduledRunAt { get; private set; } = lastScheduledRunAt;
    public DateTimeOffset? FirstSuccessfulRunAt { get; private set; } = firstSuccessfulRunAt;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = createdAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset UpdatedAt { get; private set; } = updatedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt;
    public long RowVersion { get; private set; } = rowVersion;
    public Guid? ControlTriggeredBy => null;
    public IReadOnlyList<TagSummary> Tags { get; private set; } = [];

    DateTime IAuditedEntity.CreatedAt => CreatedAt.UtcDateTime;

    public bool HasActiveRun => CurrentRunId.HasValue || ControlState == ResourceControlState.Processing;

    public void Rename(string name)
    {
        Name = BackupRepository.NormalizeName(name);
        NormalizedName = BackupRepository.ToNormalizedName(name);
        Touch();
    }

    public void UpdateDescription(string? description)
    {
        Description = BackupRepository.NormalizeOptional(description);
        Touch();
    }

    public void AssignTags(IReadOnlyList<TagSummary> tags)
    {
        Tags = tags;
    }

    public void Update(
        string? description,
        BackupSourceSpec? source,
        Guid? backupRepositoryId,
        bool? enabled,
        string? cron,
        bool updateCron,
        string? timeZone,
        bool updateTimeZone,
        BackupWebhookConfig? webhook,
        bool updateWebhook,
        int? keepLastSuccessful,
        int? timeoutSeconds,
        bool? alertOnFailure,
        Guid? runAsActorId)
    {
        if (HasActiveRun)
            throw new InvalidOperationException("Backup policy cannot be changed while a run is active.");

        Description = BackupRepository.NormalizeOptional(description);

        if (source is not null)
        {
            if (FirstSuccessfulRunAt.HasValue && !Equals(Source, NormalizeSource(source)))
                throw new InvalidOperationException("Backup policy source cannot be changed after a successful run.");

            Source = NormalizeSource(source);
        }

        if (backupRepositoryId.HasValue && backupRepositoryId.Value != Guid.Empty)
        {
            if (FirstSuccessfulRunAt.HasValue && backupRepositoryId.Value != BackupRepositoryId)
                throw new InvalidOperationException("Backup policy repository cannot be changed after a successful run.");

            BackupRepositoryId = backupRepositoryId.Value;
        }

        if (enabled.HasValue)
            Enabled = enabled.Value;

        if (updateCron)
            Cron = BackupRepository.NormalizeOptional(cron);

        if (updateTimeZone)
            TimeZone = BackupRepository.NormalizeOptional(timeZone);

        if (updateWebhook)
            Webhook = NormalizeWebhook(webhook);

        if (keepLastSuccessful.HasValue)
            KeepLastSuccessful = keepLastSuccessful.Value;

        if (timeoutSeconds.HasValue)
            TimeoutSeconds = timeoutSeconds.Value;

        if (alertOnFailure.HasValue)
            AlertOnFailure = alertOnFailure.Value;

        if (runAsActorId.HasValue && runAsActorId.Value != Guid.Empty)
            RunAsActorId = runAsActorId.Value;

        Touch();
    }

    public void MarkProcessing(Guid runId, DateTimeOffset now)
    {
        ControlState = ResourceControlState.Processing;
        CurrentRunId = runId;
        ControlStartedAt = now.ToUniversalTime().ToUnixTimeSeconds();
        Touch(now);
    }

    public void MarkIdle(Guid? runId, DateTimeOffset now)
    {
        if (runId.HasValue && CurrentRunId.HasValue && CurrentRunId != runId)
            return;

        ControlState = ResourceControlState.Idle;
        CurrentRunId = null;
        ControlStartedAt = null;
        Touch(now);
    }

    public void MarkScheduled(DateTimeOffset scheduledMinuteUtc)
    {
        LastScheduledRunAt = scheduledMinuteUtc.ToUniversalTime();
        Touch(scheduledMinuteUtc);
    }

    public void MarkFirstSuccessfulRun(DateTimeOffset now)
    {
        FirstSuccessfulRunAt ??= now.ToUniversalTime();
        Touch(now);
    }

    public void Archive(DateTimeOffset now)
    {
        if (HasActiveRun)
            throw new InvalidOperationException("Backup policy cannot be archived while a run is active.");

        Enabled = false;
        ArchivedAt ??= now.ToUniversalTime();
        Touch(now);
    }

    public void Validate()
    {
        BackupRepository.ValidateName(Name, "Backup policy");

        if (BackupRepositoryId == Guid.Empty)
            throw new ArgumentException("Backup repository is required.", nameof(BackupRepositoryId));

        if ((Cron is null) != (TimeZone is null))
            throw new ArgumentException("Backup policy cron and time zone must both be set or both be empty.");

        if (Cron?.Length > 128)
            throw new ArgumentException("Backup policy cron cannot exceed 128 characters.", nameof(Cron));

        if (TimeZone?.Length > 128)
            throw new ArgumentException("Backup policy time zone cannot exceed 128 characters.", nameof(TimeZone));

        if (Webhook?.Secret?.Length > 256)
            throw new ArgumentException("Backup policy webhook secret cannot exceed 256 characters.", nameof(Webhook));

        if (Webhook?.BranchFilter?.Length > 256)
            throw new ArgumentException("Backup policy webhook branch filter cannot exceed 256 characters.", nameof(Webhook));

        if (WebhookConfigurationValidation.GetAuthenticationError(Webhook) is { } webhookError)
            throw new ArgumentException(webhookError, nameof(Webhook));

        if (KeepLastSuccessful is < 1 or > 1000)
            throw new ArgumentException("Backup retention must keep between 1 and 1000 successful snapshots.", nameof(KeepLastSuccessful));

        if (TimeoutSeconds is < 60 or > 86_400)
            throw new ArgumentException("Backup policy timeout must be between 60 and 86400 seconds.", nameof(TimeoutSeconds));

        if (RunAsActorId == Guid.Empty)
            throw new ArgumentException("Backup policy run-as actor is required.", nameof(RunAsActorId));

        ValidateSource(Source);
    }

    public static BackupPolicy FromPersistence(
        Guid id,
        string name,
        string normalizedName,
        string? description,
        BackupSourceSpec source,
        Guid backupRepositoryId,
        bool enabled,
        string? cron,
        string? timeZone,
        BackupWebhookConfig? webhook,
        int keepLastSuccessful,
        int timeoutSeconds,
        bool alertOnFailure,
        Guid runAsActorId,
        ResourceControlState controlState,
        Guid? currentRunId,
        long? controlStartedAt,
        DateTimeOffset? lastScheduledRunAt,
        DateTimeOffset? firstSuccessfulRunAt,
        Guid createdByActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        DateTimeOffset? archivedAt,
        long rowVersion)
        => new(
            name,
            description,
            source,
            backupRepositoryId,
            enabled,
            cron,
            timeZone,
            webhook,
            keepLastSuccessful,
            timeoutSeconds,
            alertOnFailure,
            runAsActorId,
            createdByActorId,
            controlState,
            currentRunId,
            controlStartedAt,
            lastScheduledRunAt,
            firstSuccessfulRunAt,
            archivedAt,
            rowVersion,
            createdAt,
            updatedAt)
        {
            Id = id,
            NormalizedName = normalizedName
        };

    private void Touch(DateTimeOffset? now = null)
    {
        UpdatedAt = (now ?? DateTimeOffset.UtcNow).ToUniversalTime();
        RowVersion++;
    }

    private static BackupSourceSpec NormalizeSource(BackupSourceSpec source)
        => source switch
        {
            DockerVolumeBackupSource volume => volume with
            {
                VolumeName = volume.VolumeName.Trim(),
                DockerNodeId = BackupRepository.NormalizeOptional(volume.DockerNodeId)
            },
            CitadelSystemBackupSource system => system,
            StackBackupSource stack => stack,
            DeploymentBackupSource deployment => deployment,
            SwarmServiceBackupSource service => service,
            _ => throw new ArgumentException("Unsupported backup source type.", nameof(source))
        };

    private static BackupWebhookConfig? NormalizeWebhook(BackupWebhookConfig? value)
        => value is null
            ? null
            : value with
            {
                Secret = BackupRepository.NormalizeOptional(value.Secret),
                BranchFilter = BackupRepository.NormalizeOptional(value.BranchFilter)
            };

    private static void ValidateSource(BackupSourceSpec source)
    {
        switch (source)
        {
            case DockerVolumeBackupSource volume:
                if (volume.PlatformId == Guid.Empty)
                    throw new ArgumentException("Docker volume backup source requires a platform ID.", nameof(source));

                if (string.IsNullOrWhiteSpace(volume.VolumeName))
                    throw new ArgumentException("Docker volume backup source requires a named volume.", nameof(source));
                break;

            case CitadelSystemBackupSource:
                break;

            case StackBackupSource stack:
                if (stack.StackId == Guid.Empty)
                    throw new ArgumentException("Stack backup source requires a stack ID.", nameof(source));
                break;

            case DeploymentBackupSource deployment:
                if (deployment.DeploymentId == Guid.Empty)
                    throw new ArgumentException("Deployment backup source requires a deployment ID.", nameof(source));
                break;

            case SwarmServiceBackupSource service:
                if (service.SwarmServiceId == Guid.Empty)
                    throw new ArgumentException("Swarm Service backup source requires a Service ID.", nameof(source));
                break;

            default:
                throw new ArgumentException("Unsupported backup source type.", nameof(source));
        }
    }
}
