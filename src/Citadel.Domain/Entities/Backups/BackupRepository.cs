namespace Domain.Entities.Backups;

public sealed class BackupRepository(
    string name,
    string? description,
    BackupRepositorySpec spec,
    Guid passwordSecretId,
    Guid createdByActorId,
    BackupRepositoryStatus status = BackupRepositoryStatus.Unknown,
    ResourceControlState controlState = ResourceControlState.Idle,
    Guid? currentRunId = null,
    long? controlStartedAt = null,
    DateTimeOffset? lastPrunedAt = null,
    DateTimeOffset? lastCheckedAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null) : IAuditedEntity, IReconcilableResource
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = NormalizeName(name);
    public string NormalizedName { get; private set; } = ToNormalizedName(name);
    public string? Description { get; private set; } = NormalizeOptional(description);
    public BackupRepositoryType Type { get; private set; } = spec.Type;
    public BackupRepositorySpec Spec { get; private set; } = NormalizeSpec(spec);
    public Guid PasswordSecretId { get; private set; } = passwordSecretId;
    public BackupRepositoryStatus Status { get; private set; } = status;
    public ResourceControlState ControlState { get; private set; } = controlState;
    public Guid? CurrentRunId { get; private set; } = currentRunId;
    public long? ControlStartedAt { get; private set; } = controlStartedAt;
    public DateTimeOffset? LastPrunedAt { get; private set; } = lastPrunedAt;
    public DateTimeOffset? LastCheckedAt { get; private set; } = lastCheckedAt;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = createdAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset UpdatedAt { get; private set; } = updatedAt ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt;
    public long RowVersion { get; private set; } = rowVersion;
    public Guid? ControlTriggeredBy => null;

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
        ResourceControlState controlState,
        Guid? currentRunId,
        long? controlStartedAt,
        DateTimeOffset? lastPrunedAt,
        DateTimeOffset? lastCheckedAt,
        Guid createdByActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        DateTimeOffset? archivedAt,
        long rowVersion)
        => new(name, description, spec, passwordSecretId, createdByActorId, status, controlState, currentRunId, controlStartedAt, lastPrunedAt, lastCheckedAt, archivedAt, rowVersion, createdAt, updatedAt)
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
