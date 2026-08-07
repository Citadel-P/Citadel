using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Tags;

namespace Domain.Entities.Builds;

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
    BuildProjectBuilderKind builderKind = BuildProjectBuilderKind.Platform,
    Guid? buildAgentPoolId = null,
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
    public BuildProjectBuilderKind BuilderKind { get; private set; } = builderKind;
    public Guid? BuildAgentPoolId { get; private set; } = buildAgentPoolId;
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
        BuildProjectBuilderKind? builderKind,
        Guid? buildAgentPoolId,
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
        if (builderKind.HasValue) BuilderKind = builderKind.Value;
        if (BuilderKind == BuildProjectBuilderKind.Platform)
        {
            if (platformId.HasValue) PlatformId = platformId.Value;
            BuildAgentPoolId = null;
        }
        else if (BuilderKind == BuildProjectBuilderKind.BuildAgentPool)
        {
            PlatformId = Guid.Empty;
            if (buildAgentPoolId.HasValue) BuildAgentPoolId = buildAgentPoolId.Value == Guid.Empty ? null : buildAgentPoolId.Value;
        }
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
        if (BuilderKind == BuildProjectBuilderKind.Platform)
        {
            if (PlatformId == Guid.Empty) throw new ArgumentException("Platform is required.", nameof(PlatformId));
            if (BuildAgentPoolId.HasValue) throw new ArgumentException("Build pool must be empty when Docker platform builder is selected.", nameof(BuildAgentPoolId));
        }
        else if (BuilderKind == BuildProjectBuilderKind.BuildAgentPool)
        {
            if (!BuildAgentPoolId.HasValue || BuildAgentPoolId.Value == Guid.Empty) throw new ArgumentException("Build pool is required.", nameof(BuildAgentPoolId));
            if (PlatformId != Guid.Empty) throw new ArgumentException("Platform must be empty when Build Pool builder is selected.", nameof(PlatformId));
        }
        else
        {
            throw new ArgumentException("Build project builder is invalid.", nameof(BuilderKind));
        }
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
        if (WebhookConfigurationValidation.GetAuthenticationError(Webhook) is { } webhookError)
            throw new ArgumentException(webhookError, nameof(Webhook));
        ValidateBuildSecrets();
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
        BuildProjectBuilderKind builderKind,
        Guid? buildAgentPoolId,
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
            builderKind,
            buildAgentPoolId,
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
    public static bool IsBuildKitSecretId(string? value)
        => !string.IsNullOrWhiteSpace(value)
           && value.All(static ch => char.IsLetterOrDigit(ch) || ch is '.' or '_' or '-');

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
        => values?.Select(static value => value with { Id = NormalizeOptional(value.Id) ?? string.Empty }).ToArray() ?? [];

    private void ValidateBuildSecrets()
    {
        var ids = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var secret in BuildSecrets)
        {
            if (!IsBuildKitSecretId(secret.Id))
                throw new ArgumentException($"Build secret id '{secret.Id}' is invalid. Use letters, numbers, '.', '_' or '-'.", nameof(BuildSecrets));

            if (secret.SecretId == Guid.Empty)
                throw new ArgumentException($"Build secret '{secret.Id}' must reference a Citadel secret.", nameof(BuildSecrets));

            if (!ids.Add(secret.Id.Trim()))
                throw new ArgumentException($"Build secret id '{secret.Id}' is mapped more than once.", nameof(BuildSecrets));
        }
    }

}
public sealed record BuildWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);
