using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Tags;

namespace Domain.Entities.Builds;

public sealed record BuildArgSpec(
    string Name,
    string? Value = null,
    Guid? ResourceBindingId = null);

public sealed record BuildSecretSpec(
    string Id,
    Guid SecretId);

public sealed record BuildPlatformSnapshot(
    Guid Id,
    string Name,
    string Address,
    PlatformConnectorType ConnectorType);

public sealed record BuildRegistrySnapshot(
    Guid Id,
    string Name,
    string RegistryHost);

public sealed record BuildRunPlatformResult(
    string? Digest,
    IReadOnlyList<string> ImageReferences,
    int ExitCode,
    string? RawMetadata = null);

public sealed class BuildProject(
    string name,
    string? description,
    bool enabled,
    Guid gitRepositoryId,
    string branch,
    string contextPath,
    string dockerfilePath,
    string? target,
    IReadOnlyList<BuildArgSpec>? buildArgs,
    IReadOnlyList<BuildSecretSpec>? buildSecrets,
    Guid platformId,
    Guid registryId,
    string imageRepository,
    IReadOnlyList<string>? tagTemplates,
    BuildWebhookConfig? webhook,
    int timeoutSeconds,
    int retentionRunCount,
    Guid createdByActorId,
    Guid? currentRunId = null,
    ResourceControlState controlState = ResourceControlState.Idle,
    long? controlStartedAt = null,
    DateTimeOffset? createdAt = null,
    DateTimeOffset? updatedAt = null,
    DateTimeOffset? archivedAt = null,
    long rowVersion = 0) : IReconcilableResource
{
    public const int DefaultTimeoutSeconds = 1800;
    public const int DefaultRetentionRunCount = 20;

    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = NormalizeName(name);
    public string NormalizedName { get; private set; } = ToNormalizedName(name);
    public string? Description { get; private set; } = NormalizeOptional(description);
    public bool Enabled { get; private set; } = enabled;
    public Guid GitRepositoryId { get; private set; } = gitRepositoryId;
    public string Branch { get; private set; } = NormalizeName(branch);
    public string ContextPath { get; private set; } = NormalizePath(contextPath, ".");
    public string DockerfilePath { get; private set; } = NormalizePath(dockerfilePath, "Dockerfile");
    public string? Target { get; private set; } = NormalizeOptional(target);
    public IReadOnlyList<BuildArgSpec> BuildArgs { get; private set; } = NormalizeBuildArgs(buildArgs);
    public IReadOnlyList<BuildSecretSpec> BuildSecrets { get; private set; } = NormalizeBuildSecrets(buildSecrets);
    public Guid PlatformId { get; private set; } = platformId;
    public Guid RegistryId { get; private set; } = registryId;
    public string ImageRepository { get; private set; } = NormalizeImageRepository(imageRepository);
    public IReadOnlyList<string> TagTemplates { get; private set; } = NormalizeTagTemplates(tagTemplates);
    public BuildWebhookConfig? Webhook { get; private set; } = NormalizeWebhook(webhook);
    public bool WebhookEnabled => Webhook?.Enabled == true;
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public int RetentionRunCount { get; private set; } = retentionRunCount;
    public Guid? CurrentRunId { get; private set; } = currentRunId;
    public ResourceControlState ControlState { get; private set; } = controlState;
    public long? ControlStartedAt { get; private set; } = controlStartedAt;
    public Guid? ControlTriggeredBy => null;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTimeOffset CreatedAt { get; private set; } = (createdAt ?? DateTimeOffset.UtcNow).ToUniversalTime();
    public DateTimeOffset UpdatedAt { get; private set; } = (updatedAt ?? DateTimeOffset.UtcNow).ToUniversalTime();
    public DateTimeOffset? ArchivedAt { get; private set; } = archivedAt?.ToUniversalTime();
    public long RowVersion { get; private set; } = rowVersion;
    public IReadOnlyList<TagSummary> Tags { get; private set; } = [];

    public bool HasActiveRun => CurrentRunId.HasValue || ControlState == ResourceControlState.Processing;

    public void AssignTags(IReadOnlyList<TagSummary> tags) => Tags = tags;

    public void Rename(string name)
    {
        Name = NormalizeName(name);
        NormalizedName = ToNormalizedName(name);
        Touch();
    }

    public void UpdateDescription(string? description)
    {
        Description = NormalizeOptional(description);
        Touch();
    }

    public void Update(
        string? description,
        bool? enabled,
        Guid? gitRepositoryId,
        string? branch,
        string? contextPath,
        string? dockerfilePath,
        string? target,
        IReadOnlyList<BuildArgSpec>? buildArgs,
        bool updateBuildArgs,
        IReadOnlyList<BuildSecretSpec>? buildSecrets,
        bool updateBuildSecrets,
        Guid? platformId,
        Guid? registryId,
        string? imageRepository,
        IReadOnlyList<string>? tagTemplates,
        BuildWebhookConfig? webhook,
        bool updateWebhook,
        int? timeoutSeconds,
        int? retentionRunCount)
    {
        if (HasActiveRun)
            throw new InvalidOperationException("Build project cannot be changed while a run is active.");

        Description = NormalizeOptional(description);
        if (enabled.HasValue) Enabled = enabled.Value;
        if (gitRepositoryId.HasValue && gitRepositoryId.Value != Guid.Empty) GitRepositoryId = gitRepositoryId.Value;
        if (branch is not null) Branch = NormalizeName(branch);
        if (contextPath is not null) ContextPath = NormalizePath(contextPath, ".");
        if (dockerfilePath is not null) DockerfilePath = NormalizePath(dockerfilePath, "Dockerfile");
        if (target is not null) Target = NormalizeOptional(target);
        if (updateBuildArgs) BuildArgs = NormalizeBuildArgs(buildArgs);
        if (updateBuildSecrets) BuildSecrets = NormalizeBuildSecrets(buildSecrets);
        if (platformId.HasValue && platformId.Value != Guid.Empty) PlatformId = platformId.Value;
        if (registryId.HasValue && registryId.Value != Guid.Empty) RegistryId = registryId.Value;
        if (imageRepository is not null) ImageRepository = NormalizeImageRepository(imageRepository);
        if (tagTemplates is not null) TagTemplates = NormalizeTagTemplates(tagTemplates);
        if (updateWebhook) Webhook = NormalizeWebhook(webhook);
        if (timeoutSeconds.HasValue) TimeoutSeconds = timeoutSeconds.Value;
        if (retentionRunCount.HasValue) RetentionRunCount = retentionRunCount.Value;
        Touch();
    }

    public void MarkProcessing(Guid runId, DateTimeOffset now)
    {
        CurrentRunId = runId;
        ControlState = ResourceControlState.Processing;
        ControlStartedAt = now.ToUniversalTime().ToUnixTimeSeconds();
        Touch(now);
    }

    public void MarkIdle(Guid runId, DateTimeOffset now)
    {
        if (CurrentRunId == runId)
            CurrentRunId = null;

        ControlState = ResourceControlState.Idle;
        ControlStartedAt = null;
        Touch(now);
    }

    public void Archive(DateTimeOffset now)
    {
        if (HasActiveRun)
            throw new InvalidOperationException("Build project cannot be archived while a run is active.");

        Enabled = false;
        ArchivedAt ??= now.ToUniversalTime();
        Touch(now);
    }

    public void Validate()
    {
        ValidateName(Name, "Build project");
        if (GitRepositoryId == Guid.Empty) throw new ArgumentException("Git repository is required.", nameof(GitRepositoryId));
        if (PlatformId == Guid.Empty) throw new ArgumentException("Platform is required.", nameof(PlatformId));
        if (RegistryId == Guid.Empty) throw new ArgumentException("Registry is required.", nameof(RegistryId));
        if (Branch.Length > 256) throw new ArgumentException("Branch cannot exceed 256 characters.", nameof(Branch));
        if (ContextPath.Length > 512) throw new ArgumentException("Context path cannot exceed 512 characters.", nameof(ContextPath));
        if (DockerfilePath.Length > 512) throw new ArgumentException("Dockerfile path cannot exceed 512 characters.", nameof(DockerfilePath));
        if (Target?.Length > 128) throw new ArgumentException("Target cannot exceed 128 characters.", nameof(Target));
        if (ImageRepository.Length > 512) throw new ArgumentException("Image repository cannot exceed 512 characters.", nameof(ImageRepository));
        if (TimeoutSeconds is < 60 or > 86_400) throw new ArgumentException("Build timeout must be between 60 and 86400 seconds.", nameof(TimeoutSeconds));
        if (RetentionRunCount is < 1 or > 1000) throw new ArgumentException("Build retention must keep between 1 and 1000 runs.", nameof(RetentionRunCount));
        if (TagTemplates.Count == 0) throw new ArgumentException("At least one tag template is required.", nameof(TagTemplates));
        if (Webhook?.Secret?.Length > 256) throw new ArgumentException("Build webhook secret cannot exceed 256 characters.", nameof(Webhook));
        if (Webhook?.BranchFilter?.Length > 256) throw new ArgumentException("Build webhook branch filter cannot exceed 256 characters.", nameof(Webhook));
    }

    public static BuildProject FromPersistence(
        Guid id,
        string name,
        string normalizedName,
        string? description,
        bool enabled,
        Guid gitRepositoryId,
        string branch,
        string contextPath,
        string dockerfilePath,
        string? target,
        IReadOnlyList<BuildArgSpec> buildArgs,
        IReadOnlyList<BuildSecretSpec> buildSecrets,
        Guid platformId,
        Guid registryId,
        string imageRepository,
        IReadOnlyList<string> tagTemplates,
        BuildWebhookConfig? webhook,
        int timeoutSeconds,
        int retentionRunCount,
        Guid? currentRunId,
        ResourceControlState controlState,
        long? controlStartedAt,
        Guid createdByActorId,
        DateTimeOffset createdAt,
        DateTimeOffset updatedAt,
        DateTimeOffset? archivedAt,
        long rowVersion)
        => new(
            name,
            description,
            enabled,
            gitRepositoryId,
            branch,
            contextPath,
            dockerfilePath,
            target,
            buildArgs,
            buildSecrets,
            platformId,
            registryId,
            imageRepository,
            tagTemplates,
            webhook,
            timeoutSeconds,
            retentionRunCount,
            createdByActorId,
            currentRunId,
            controlState,
            controlStartedAt,
            createdAt,
            updatedAt,
            archivedAt,
            rowVersion)
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
    public static string? NormalizeOptional(string? value) => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    public static void ValidateName(string value, string label)
    {
        if (string.IsNullOrWhiteSpace(value))
            throw new ArgumentException($"{label} name is required.", nameof(value));

        if (value.Trim().Length > 128)
            throw new ArgumentException($"{label} name cannot exceed 128 characters.", nameof(value));
    }

    private static string NormalizePath(string value, string fallback)
    {
        var normalized = string.IsNullOrWhiteSpace(value) ? fallback : value.Trim().Replace('\\', '/');
        return normalized.TrimStart('/');
    }

    private static string NormalizeImageRepository(string value)
        => string.IsNullOrWhiteSpace(value)
            ? throw new ArgumentException("Image repository is required.", nameof(value))
            : value.Trim().TrimStart('/');

    private static IReadOnlyList<string> NormalizeTagTemplates(IReadOnlyList<string>? values)
        => (values is null || values.Count == 0 ? ["{branch}-{shortSha}"] : values)
            .Select(static value => value.Trim())
            .Where(static value => value.Length > 0)
            .Distinct(StringComparer.Ordinal)
            .ToArray();

    private static BuildWebhookConfig? NormalizeWebhook(BuildWebhookConfig? value)
        => value is null
            ? null
            : value with
            {
                Secret = NormalizeOptional(value.Secret),
                BranchFilter = NormalizeOptional(value.BranchFilter)
            };

    private static IReadOnlyList<BuildArgSpec> NormalizeBuildArgs(IReadOnlyList<BuildArgSpec>? values)
        => values?.Select(static value => value with
            {
                Name = NormalizeName(value.Name),
                Value = NormalizeOptional(value.Value)
            }).ToArray() ?? [];

    private static IReadOnlyList<BuildSecretSpec> NormalizeBuildSecrets(IReadOnlyList<BuildSecretSpec>? values)
        => values?.Select(static value => value with { Id = NormalizeName(value.Id) }).ToArray() ?? [];
}

public sealed class BuildRun(
    Guid buildProjectId,
    string projectNameSnapshot,
    Guid gitRepositoryId,
    string gitRepositoryNameSnapshot,
    string branch,
    string? resolvedCommitSha,
    string contextPath,
    string dockerfilePath,
    string? target,
    IReadOnlyList<BuildArgSpec> buildArgsSnapshot,
    IReadOnlyList<string> buildSecretIdsSnapshot,
    BuildPlatformSnapshot platformSnapshot,
    BuildRegistrySnapshot registrySnapshot,
    string imageRepository,
    IReadOnlyList<string> tagTemplatesSnapshot,
    IReadOnlyList<string> imageReferences,
    BuildRunTrigger trigger,
    Guid? triggerSourceId,
    Guid triggeredByActorId,
    int timeoutSeconds,
    DateTimeOffset? queuedAt = null,
    BuildRunStatus status = BuildRunStatus.Queued,
    string? imageDigest = null,
    DateTimeOffset? startedAt = null,
    DateTimeOffset? completedAt = null,
    int? exitCode = null,
    string? errorCode = null,
    string? errorMessage = null)
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid BuildProjectId { get; private set; } = buildProjectId;
    public string ProjectNameSnapshot { get; private set; } = BuildProject.NormalizeName(projectNameSnapshot);
    public Guid GitRepositoryId { get; private set; } = gitRepositoryId;
    public string GitRepositoryNameSnapshot { get; private set; } = BuildProject.NormalizeName(gitRepositoryNameSnapshot);
    public string Branch { get; private set; } = BuildProject.NormalizeName(branch);
    public string? ResolvedCommitSha { get; private set; } = BuildProject.NormalizeOptional(resolvedCommitSha);
    public string ContextPath { get; private set; } = contextPath;
    public string DockerfilePath { get; private set; } = dockerfilePath;
    public string? Target { get; private set; } = BuildProject.NormalizeOptional(target);
    public IReadOnlyList<BuildArgSpec> BuildArgsSnapshot { get; private set; } = buildArgsSnapshot;
    public IReadOnlyList<string> BuildSecretIdsSnapshot { get; private set; } = buildSecretIdsSnapshot;
    public BuildPlatformSnapshot PlatformSnapshot { get; private set; } = platformSnapshot;
    public BuildRegistrySnapshot RegistrySnapshot { get; private set; } = registrySnapshot;
    public string ImageRepository { get; private set; } = imageRepository;
    public IReadOnlyList<string> TagTemplatesSnapshot { get; private set; } = tagTemplatesSnapshot;
    public IReadOnlyList<string> ImageReferences { get; private set; } = imageReferences;
    public BuildRunTrigger Trigger { get; private set; } = trigger;
    public Guid? TriggerSourceId { get; private set; } = triggerSourceId;
    public BuildRunStatus Status { get; private set; } = status;
    public string? ImageDigest { get; private set; } = BuildProject.NormalizeOptional(imageDigest);
    public int TimeoutSeconds { get; private set; } = timeoutSeconds;
    public DateTimeOffset QueuedAt { get; private set; } = queuedAt?.ToUniversalTime() ?? DateTimeOffset.UtcNow;
    public DateTimeOffset? StartedAt { get; private set; } = startedAt?.ToUniversalTime();
    public DateTimeOffset? CompletedAt { get; private set; } = completedAt?.ToUniversalTime();
    public int? ExitCode { get; private set; } = exitCode;
    public string? ErrorCode { get; private set; } = BuildProject.NormalizeOptional(errorCode);
    public string? ErrorMessage { get; private set; } = BuildProject.NormalizeOptional(errorMessage);
    public Guid TriggeredByActorId { get; private set; } = triggeredByActorId;

    public bool IsActive => Status is BuildRunStatus.Queued or BuildRunStatus.Preparing or BuildRunStatus.Running;

    public void MarkPreparing(DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Queued);
        Status = BuildRunStatus.Preparing;
        StartedAt ??= now.ToUniversalTime();
    }

    public void MarkRunning(DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Queued, BuildRunStatus.Preparing);
        Status = BuildRunStatus.Running;
        StartedAt ??= now.ToUniversalTime();
    }

    public void ResolveCommit(string commitSha, IReadOnlyList<string>? imageReferences = null)
    {
        ResolvedCommitSha = BuildProject.NormalizeOptional(commitSha);
        if (imageReferences is not null)
            ImageReferences = imageReferences;
    }

    public void CompleteSucceeded(string? imageDigest, IReadOnlyList<string> imageReferences, int? exitCode, DateTimeOffset now)
    {
        EnsureStatus(BuildRunStatus.Running, BuildRunStatus.Preparing);
        ImageDigest = BuildProject.NormalizeOptional(imageDigest);
        ImageReferences = imageReferences;
        Complete(BuildRunStatus.Succeeded, exitCode, null, null, now);
    }

    public void Fail(BuildRunStatus status, int? exitCode, string? errorCode, string errorMessage, DateTimeOffset now)
    {
        if (status is not (BuildRunStatus.Failed or BuildRunStatus.TimedOut or BuildRunStatus.Interrupted))
            throw new ArgumentException("Build run failure status is invalid.", nameof(status));

        Complete(status, exitCode, errorCode, errorMessage, now);
    }

    public void Cancel(DateTimeOffset now)
    {
        if (!IsActive)
            throw new InvalidOperationException($"Build run cannot be cancelled from status {Status}.");

        Complete(BuildRunStatus.Cancelled, null, "build.cancelled", "Build run cancelled.", now);
    }

    public static BuildRun FromPersistence(
        Guid id,
        Guid buildProjectId,
        string projectNameSnapshot,
        Guid gitRepositoryId,
        string gitRepositoryNameSnapshot,
        string branch,
        string? resolvedCommitSha,
        string contextPath,
        string dockerfilePath,
        string? target,
        IReadOnlyList<BuildArgSpec> buildArgsSnapshot,
        IReadOnlyList<string> buildSecretIdsSnapshot,
        BuildPlatformSnapshot platformSnapshot,
        BuildRegistrySnapshot registrySnapshot,
        string imageRepository,
        IReadOnlyList<string> tagTemplatesSnapshot,
        IReadOnlyList<string> imageReferences,
        BuildRunTrigger trigger,
        Guid? triggerSourceId,
        BuildRunStatus status,
        string? imageDigest,
        int timeoutSeconds,
        DateTimeOffset queuedAt,
        DateTimeOffset? startedAt,
        DateTimeOffset? completedAt,
        int? exitCode,
        string? errorCode,
        string? errorMessage,
        Guid triggeredByActorId)
        => new(
            buildProjectId,
            projectNameSnapshot,
            gitRepositoryId,
            gitRepositoryNameSnapshot,
            branch,
            resolvedCommitSha,
            contextPath,
            dockerfilePath,
            target,
            buildArgsSnapshot,
            buildSecretIdsSnapshot,
            platformSnapshot,
            registrySnapshot,
            imageRepository,
            tagTemplatesSnapshot,
            imageReferences,
            trigger,
            triggerSourceId,
            triggeredByActorId,
            timeoutSeconds,
            queuedAt,
            status,
            imageDigest,
            startedAt,
            completedAt,
            exitCode,
            errorCode,
            errorMessage)
        {
            Id = id
        };

    private void Complete(BuildRunStatus status, int? exitCode, string? errorCode, string? errorMessage, DateTimeOffset now)
    {
        Status = status;
        ExitCode = exitCode;
        ErrorCode = BuildProject.NormalizeOptional(errorCode);
        ErrorMessage = BuildProject.NormalizeOptional(errorMessage);
        CompletedAt = now.ToUniversalTime();
    }

    private void EnsureStatus(params BuildRunStatus[] statuses)
    {
        if (!statuses.Contains(Status))
            throw new InvalidOperationException($"Build run cannot transition from {Status}.");
    }
}

public sealed record BuildRunLogEntry(
    Guid Id,
    Guid BuildRunId,
    DateTimeOffset CreatedAt,
    string Stream,
    string Message);

public sealed record BuildWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);
