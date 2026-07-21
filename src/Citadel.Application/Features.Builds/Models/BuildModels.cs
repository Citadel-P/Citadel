using Domain;
using Domain.Entities.Builds;

namespace Application.Features.Builds.Models;

public sealed record BuildProjectInputModel(
    string Name,
    string? Description,
    bool Enabled,
    Guid GitRepositoryId,
    string? Branch,
    string? ContextPath,
    string? DockerfilePath,
    string? Target,
    IReadOnlyList<BuildArgSpec>? BuildArgs,
    IReadOnlyList<BuildSecretSpec>? BuildSecrets,
    Guid? PlatformId,
    Guid RegistryId,
    string ImageRepository,
    IReadOnlyList<string>? TagTemplates,
    BuildWebhookConfig? Webhook,
    int? TimeoutSeconds,
    int? RetentionRunCount,
    IReadOnlyCollection<Guid>? TagIds = null,
    BuildProjectBuilderKind BuilderKind = BuildProjectBuilderKind.Platform,
    Guid? BuildAgentPoolId = null);

public sealed record UpdateBuildProjectInputModel(
    string? Description = null,
    bool? Enabled = null,
    Guid? GitRepositoryId = null,
    string? Branch = null,
    string? ContextPath = null,
    string? DockerfilePath = null,
    string? Target = null,
    IReadOnlyList<BuildArgSpec>? BuildArgs = null,
    IReadOnlyList<BuildSecretSpec>? BuildSecrets = null,
    BuildProjectBuilderKind? BuilderKind = null,
    Guid? PlatformId = null,
    Guid? BuildAgentPoolId = null,
    Guid? RegistryId = null,
    string? ImageRepository = null,
    IReadOnlyList<string>? TagTemplates = null,
    BuildWebhookConfig? Webhook = null,
    int? TimeoutSeconds = null,
    int? RetentionRunCount = null);

public sealed record QueueBuildRunInputModel(
    BuildRunTrigger Trigger = BuildRunTrigger.Manual,
    Guid? TriggerSourceId = null);

public sealed record BuildProjectResult(BuildProject Project, BuildRun? LatestRun = null);

public sealed record BuildProjectListResult(
    IReadOnlyList<BuildProject> Projects,
    IReadOnlyDictionary<Guid, BuildRun> LatestRuns);

public sealed record BuildRunResult(BuildRun Run);

public sealed record BuildRunListResult(IReadOnlyList<BuildRun> Runs);

public sealed record BuildRunLogResult(Guid RunId, IReadOnlyList<BuildRunLogEntry> Logs);

public sealed record BuildAgentPoolInputModel(
    string Name,
    string? Description,
    bool Enabled,
    BuildAgentPoolProviderSpec ProviderSpec,
    int? MaxActiveBuilders,
    int? QueueTimeoutSeconds,
    int? ProvisioningTimeoutSeconds,
    int? RegistrationTimeoutSeconds,
    int? HeartbeatTimeoutSeconds,
    int? CleanupTimeoutSeconds,
    int? MaximumInstanceLifetimeSeconds,
    int? FailureRetentionMinutes,
    IReadOnlyCollection<Guid>? TagIds = null);

public sealed record UpdateBuildAgentPoolInputModel(
    string? Description = null,
    bool? Enabled = null,
    BuildAgentPoolProviderSpec? ProviderSpec = null,
    int? MaxActiveBuilders = null,
    int? QueueTimeoutSeconds = null,
    int? ProvisioningTimeoutSeconds = null,
    int? RegistrationTimeoutSeconds = null,
    int? HeartbeatTimeoutSeconds = null,
    int? CleanupTimeoutSeconds = null,
    int? MaximumInstanceLifetimeSeconds = null,
    int? FailureRetentionMinutes = null);

public sealed record BuildAgentPoolResult(BuildAgentPool Pool);

public sealed record BuildAgentPoolListResult(IReadOnlyList<BuildAgentPool> Pools);
