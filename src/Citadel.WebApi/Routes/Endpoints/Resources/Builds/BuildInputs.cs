using Application.Features.Builds.Models;
using Domain;
using Domain.Entities.Builds;

namespace WebApi.Routes.Endpoints.Resources.Builds;

public sealed record BuildProjectInput(
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
    Guid? BuildAgentPoolId = null)
{
    internal BuildProjectInputModel ToModel()
        => new(
            Name,
            Description,
            Enabled,
            GitRepositoryId,
            Branch,
            ContextPath,
            DockerfilePath,
            Target,
            BuildArgs,
            BuildSecrets,
            PlatformId,
            RegistryId,
            ImageRepository,
            TagTemplates,
            Webhook,
            TimeoutSeconds,
            RetentionRunCount,
            TagIds,
            BuilderKind,
            BuildAgentPoolId);
}

public sealed record UpdateBuildProjectInput(
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
    int? RetentionRunCount = null)
{
    internal UpdateBuildProjectInputModel ToModel()
        => new(
            Description,
            Enabled,
            GitRepositoryId,
            Branch,
            ContextPath,
            DockerfilePath,
            Target,
            BuildArgs,
            BuildSecrets,
            BuilderKind,
            PlatformId,
            BuildAgentPoolId,
            RegistryId,
            ImageRepository,
            TagTemplates,
            Webhook,
            TimeoutSeconds,
            RetentionRunCount);
}

public sealed record QueueBuildRunInput(
    BuildRunTrigger Trigger = BuildRunTrigger.Manual,
    Guid? TriggerSourceId = null)
{
    internal QueueBuildRunInputModel ToModel() => new(Trigger, TriggerSourceId);
}

public sealed record BuildAgentPoolInput(
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
    IReadOnlyCollection<Guid>? TagIds)
{
    internal BuildAgentPoolInputModel ToModel()
        => new(
            Name,
            Description,
            Enabled,
            ProviderSpec,
            MaxActiveBuilders,
            QueueTimeoutSeconds,
            ProvisioningTimeoutSeconds,
            RegistrationTimeoutSeconds,
            HeartbeatTimeoutSeconds,
            CleanupTimeoutSeconds,
            MaximumInstanceLifetimeSeconds,
            FailureRetentionMinutes,
            TagIds);
}

public sealed record UpdateBuildAgentPoolInput(
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
    int? FailureRetentionMinutes = null)
{
    internal UpdateBuildAgentPoolInputModel ToModel()
        => new(
            Description,
            Enabled,
            ProviderSpec,
            MaxActiveBuilders,
            QueueTimeoutSeconds,
            ProvisioningTimeoutSeconds,
            RegistrationTimeoutSeconds,
            HeartbeatTimeoutSeconds,
            CleanupTimeoutSeconds,
            MaximumInstanceLifetimeSeconds,
            FailureRetentionMinutes);
}
