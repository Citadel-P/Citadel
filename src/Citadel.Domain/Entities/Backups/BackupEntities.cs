using Domain.Entities;
using Domain.Entities.Tags;
using System.Text.Json.Serialization;

namespace Domain.Entities.Backups;

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(DockerVolumeBackupSource), "DockerVolume")]
[JsonDerivedType(typeof(CitadelSystemBackupSource), "CitadelSystem")]
[JsonDerivedType(typeof(StackBackupSource), "Stack")]
[JsonDerivedType(typeof(DeploymentBackupSource), "Deployment")]
public abstract record BackupSourceSpec
{
    public abstract BackupSourceType Type { get; }
    public abstract string StableKey { get; }
}

public sealed record DockerVolumeBackupSource(
    Guid PlatformId,
    string VolumeName,
    VolumeBackupConsistency Consistency = VolumeBackupConsistency.Live) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.DockerVolume;
    public override string StableKey => $"{PlatformId}:{VolumeName}";
}

public sealed record CitadelSystemBackupSource() : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.CitadelSystem;
    public override string StableKey => "citadel-system";
}

public sealed record StackBackupSource(Guid StackId) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.Stack;
    public override string StableKey => $"stack:{StackId}";
}

public sealed record DeploymentBackupSource(Guid DeploymentId) : BackupSourceSpec
{
    public override BackupSourceType Type => BackupSourceType.Deployment;
    public override string StableKey => $"deployment:{DeploymentId}";
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "$type")]
[JsonDerivedType(typeof(FileSystemBackupRepositorySpec), "FileSystem")]
[JsonDerivedType(typeof(S3CompatibleBackupRepositorySpec), "S3Compatible")]
public abstract record BackupRepositorySpec
{
    public abstract BackupRepositoryType Type { get; }
}

public sealed record FileSystemBackupRepositorySpec(
    BackupExecutionLocation Location,
    Guid? PlatformId,
    string Path) : BackupRepositorySpec
{
    public override BackupRepositoryType Type => BackupRepositoryType.FileSystem;
}

public sealed record S3CompatibleBackupRepositorySpec(
    Uri Endpoint,
    string Bucket,
    string? Prefix,
    string? Region,
    S3BucketLookup BucketLookup,
    Guid AccessKeySecretId,
    Guid SecretKeySecretId,
    Guid? SessionTokenSecretId,
    bool AllowInsecureHttp = false) : BackupRepositorySpec
{
    public override BackupRepositoryType Type => BackupRepositoryType.S3Compatible;
}

public sealed record BackupExecutionContext(BackupExecutionLocation Location, Guid? PlatformId)
{
    public void Validate()
    {
        if (Location == BackupExecutionLocation.Core && PlatformId.HasValue)
            throw new ArgumentException("Core backup execution cannot include a platform ID.", nameof(PlatformId));

        if (Location == BackupExecutionLocation.Platform && (!PlatformId.HasValue || PlatformId.Value == Guid.Empty))
            throw new ArgumentException("Platform backup execution requires a platform ID.", nameof(PlatformId));
    }
}

public sealed class BackupRepository(
    string name,
    string? description,
    BackupRepositorySpec spec,
    Guid passwordSecretId,
    Guid createdByActorId,
    BackupRepositoryStatus status = BackupRepositoryStatus.Unknown,
    DateTimeOffset? lastPrunedAt = null,
    DateTimeOffset? lastCheckedAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = NormalizeName(name);
    public string NormalizedName { get; private set; } = ToNormalizedName(name);
    public string? Description { get; private set; } = NormalizeOptional(description);
    public BackupRepositoryType Type { get; private set; } = spec.Type;
    public BackupRepositorySpec Spec { get; private set; } = NormalizeSpec(spec);
    public Guid PasswordSecretId { get; private set; } = passwordSecretId;
    public BackupRepositoryStatus Status { get; private set; } = status;
    public DateTimeOffset? LastPrunedAt { get; private set; } = lastPrunedAt;
    public DateTimeOffset? LastCheckedAt { get; private set; } = lastCheckedAt;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = createdAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset UpdatedAt { get; private set; } = updatedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt;
    public long RowVersion { get; private set; } = rowVersion;

    DateTime IAuditedEntity.CreatedAt => CreatedAt.UtcDateTime;

    public void Rename(string name)
    {
        Name = NormalizeName(name);
        NormalizedName = ToNormalizedName(name);
        Touch();
    }

    public void Update(string? description, BackupRepositorySpec? spec)
    {
        Description = NormalizeOptional(description);

        if (spec is not null)
        {
            if (Status == BackupRepositoryStatus.Ready && !Equals(Spec, NormalizeSpec(spec)))
                throw new InvalidOperationException("Ready backup repository location cannot be changed.");

            Spec = NormalizeSpec(spec);
            Type = Spec.Type;
        }

        Touch();
    }

    public void MarkReady(DateTimeOffset now)
    {
        Status = BackupRepositoryStatus.Ready;
        Touch(now);
    }

    public void MarkUninitialized(DateTimeOffset now)
    {
        Status = BackupRepositoryStatus.Uninitialized;
        Touch(now);
    }

    public void MarkChecked(DateTimeOffset now)
    {
        LastCheckedAt = now.ToUniversalTime();
        Touch(now);
    }

    public void MarkPruned(DateTimeOffset now)
    {
        LastPrunedAt = now.ToUniversalTime();
        Touch(now);
    }

    public void Archive(DateTimeOffset now)
    {
        ArchivedAt ??= now.ToUniversalTime();
        Touch(now);
    }

    public void Validate()
    {
        ValidateName(Name, "Backup repository");

        if (PasswordSecretId == Guid.Empty)
            throw new ArgumentException("Backup repository password secret is required.", nameof(PasswordSecretId));

        ValidateSpec(Spec);
    }

    public static BackupRepository FromPersistence(
        Guid id,
        string name,
        string normalizedName,
        string? description,
        BackupRepositorySpec spec,
        Guid passwordSecretId,
        BackupRepositoryStatus status,
        DateTimeOffset? lastPrunedAt,
        DateTimeOffset? lastCheckedAt,
        Guid createdByActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        DateTimeOffset? archivedAt,
        long rowVersion)
        => new(name, description, spec, passwordSecretId, createdByActorId, status, lastPrunedAt, lastCheckedAt, archivedAt, rowVersion, createdAt, updatedAt)
        {
            Id = id,
            NormalizedName = normalizedName
        };

    private void Touch(DateTimeOffset? now = null)
    {
        UpdatedAt = (now ?? DateTimeOffset.UtcNow).ToUniversalTime();
        RowVersion++;
    }

    public static string NormalizeName(string value) => value.Trim();

    public static string ToNormalizedName(string value) => NormalizeName(value).ToUpperInvariant();

    public static string? NormalizeOptional(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    internal static void ValidateName(string name, string resourceName)
    {
        if (string.IsNullOrWhiteSpace(name))
            throw new ArgumentException($"{resourceName} name is required.", nameof(name));

        if (name.Length > 128)
            throw new ArgumentException($"{resourceName} name cannot exceed 128 characters.", nameof(name));
    }

    private static BackupRepositorySpec NormalizeSpec(BackupRepositorySpec spec)
        => spec switch
        {
            FileSystemBackupRepositorySpec fs => fs with { Path = fs.Path.Trim() },
            S3CompatibleBackupRepositorySpec s3 => s3 with
            {
                Bucket = s3.Bucket.Trim(),
                Prefix = NormalizeS3Prefix(s3.Prefix),
                Region = NormalizeOptional(s3.Region)
            },
            _ => throw new ArgumentException("Unsupported backup repository type.", nameof(spec))
        };

    private static void ValidateSpec(BackupRepositorySpec spec)
    {
        switch (spec)
        {
            case FileSystemBackupRepositorySpec fs:
                new BackupExecutionContext(fs.Location, fs.PlatformId).Validate();
                if (string.IsNullOrWhiteSpace(fs.Path))
                    throw new ArgumentException("Filesystem backup repository path is required.", nameof(spec));
                break;

            case S3CompatibleBackupRepositorySpec s3:
                if (!s3.Endpoint.IsAbsoluteUri || (s3.Endpoint.Scheme != Uri.UriSchemeHttps && s3.Endpoint.Scheme != Uri.UriSchemeHttp))
                    throw new ArgumentException("S3 backup repository endpoint must be an absolute HTTP or HTTPS URI.", nameof(spec));

                if (s3.Endpoint.Scheme == Uri.UriSchemeHttp && !s3.AllowInsecureHttp)
                    throw new ArgumentException("S3 backup repository endpoint must use HTTPS unless insecure HTTP is explicitly allowed.", nameof(spec));

                if (string.IsNullOrWhiteSpace(s3.Bucket))
                    throw new ArgumentException("S3 backup repository bucket is required.", nameof(spec));

                if (s3.AccessKeySecretId == Guid.Empty || s3.SecretKeySecretId == Guid.Empty)
                    throw new ArgumentException("S3 backup repository credentials are required.", nameof(spec));
                break;

            default:
                throw new ArgumentException("Unsupported backup repository type.", nameof(spec));
        }
    }

    private static string? NormalizeS3Prefix(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim().Trim('/');
}

public sealed class BackupRepositoryValidation(
    Guid backupRepositoryId,
    BackupExecutionLocation location,
    Guid? platformId,
    BackupRepositoryValidationStatus status,
    DateTimeOffset lastValidatedAt,
    string? lastErrorCode,
    string? lastErrorMessage)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public BackupExecutionLocation Location { get; private set; } = location;
    public Guid? PlatformId { get; private set; } = platformId;
    public BackupRepositoryValidationStatus Status { get; private set; } = status;
    public DateTimeOffset LastValidatedAt { get; private set; } = lastValidatedAt.ToUniversalTime();
    public string? LastErrorCode { get; private set; } = BackupRepository.NormalizeOptional(lastErrorCode);
    public string? LastErrorMessage { get; private set; } = BackupRepository.NormalizeOptional(lastErrorMessage);

    public void Validate()
    {
        if (BackupRepositoryId == Guid.Empty)
            throw new ArgumentException("Backup repository ID is required.", nameof(BackupRepositoryId));

        new BackupExecutionContext(Location, PlatformId).Validate();
    }

    public static BackupRepositoryValidation FromPersistence(
        Guid id,
        Guid backupRepositoryId,
        BackupExecutionLocation location,
        Guid? platformId,
        BackupRepositoryValidationStatus status,
        DateTimeOffset lastValidatedAt,
        string? lastErrorCode,
        string? lastErrorMessage)
        => new(backupRepositoryId, location, platformId, status, lastValidatedAt, lastErrorCode, lastErrorMessage)
        {
            Id = id
        };
}

public sealed class BackupPolicy(
    string name,
    string? description,
    BackupSourceSpec source,
    Guid backupRepositoryId,
    bool enabled,
    string? cron,
    string? timeZone,
    int keepLastSuccessful,
    int timeoutSeconds,
    bool alertOnFailure,
    Guid runAsActorId,
    Guid createdByActorId,
    ResourceControlState controlState = ResourceControlState.Idle,
    Guid? currentRunId = null,
    DateTimeOffset? lastScheduledRunAt = null,
    DateTimeOffset? firstSuccessfulRunAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null) : IAuditedEntity
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
    public int KeepLastSuccessful { get; private set; } = keepLastSuccessful;
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public bool AlertOnFailure { get; private set; } = alertOnFailure;
    public Guid RunAsActorId { get; private set; } = runAsActorId;
    public ResourceControlState ControlState { get; private set; } = controlState;
    public Guid? CurrentRunId { get; private set; } = currentRunId;
    public DateTimeOffset? LastScheduledRunAt { get; private set; } = lastScheduledRunAt;
    public DateTimeOffset? FirstSuccessfulRunAt { get; private set; } = firstSuccessfulRunAt;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = createdAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset UpdatedAt { get; private set; } = updatedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt;
    public long RowVersion { get; private set; } = rowVersion;
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
        Touch(now);
    }

    public void MarkIdle(Guid? runId, DateTimeOffset now)
    {
        if (runId.HasValue && CurrentRunId.HasValue && CurrentRunId != runId)
            return;

        ControlState = ResourceControlState.Idle;
        CurrentRunId = null;
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
        int keepLastSuccessful,
        int timeoutSeconds,
        bool alertOnFailure,
        Guid runAsActorId,
        ResourceControlState controlState,
        Guid? currentRunId,
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
            keepLastSuccessful,
            timeoutSeconds,
            alertOnFailure,
            runAsActorId,
            createdByActorId,
            controlState,
            currentRunId,
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
            DockerVolumeBackupSource volume => volume with { VolumeName = volume.VolumeName.Trim() },
            CitadelSystemBackupSource system => system,
            StackBackupSource stack => stack,
            DeploymentBackupSource deployment => deployment,
            _ => throw new ArgumentException("Unsupported backup source type.", nameof(source))
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

            default:
                throw new ArgumentException("Unsupported backup source type.", nameof(source));
        }
    }
}

public sealed record BackupRunWarning(string Code, string Message);

public sealed record BackupAffectedContainer(
    string DockerContainerId,
    string Name,
    ContainerStateStatus OriginalState,
    bool StopAttempted,
    bool RestartAttempted,
    bool RestartSucceeded);

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
    DateTimeOffset? updatedAt = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRunId { get; private set; } = backupRunId;
    public Guid PlatformId { get; private set; } = platformId;
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
        DateTimeOffset updatedAt)
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
            updatedAt)
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
            SnapshotAvailability = BackupSnapshotAvailability.NotCreated;

        Complete(status, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (Status is not (BackupRunStatus.Queued or BackupRunStatus.Preparing or BackupRunStatus.Running or BackupRunStatus.ApplyingRetention))
            throw new InvalidOperationException($"Backup run cannot be cancelled from status {Status}.");

        if (SnapshotAvailability == BackupSnapshotAvailability.Pending)
            SnapshotAvailability = BackupSnapshotAvailability.NotCreated;

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
    string? errorMessage = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BackupRunId { get; private set; } = backupRunId;
    public Guid BackupRepositoryId { get; private set; } = backupRepositoryId;
    public BackupRestoreStatus Status { get; private set; } = status;
    public Guid TargetPlatformId { get; private set; } = targetPlatformId;
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
        Guid triggeredByActorId)
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
            errorMessage)
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
