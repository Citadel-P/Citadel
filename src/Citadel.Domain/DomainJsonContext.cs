using Domain.Contracts.Resources.Role;
using Domain.Contracts.Resources.Identity;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using System.Text.Json.Serialization;

namespace Domain;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<PlatformType>),
        typeof(JsonStringEnumConverter<PlatformStatus>),
        typeof(JsonStringEnumConverter<PlatformConnectorType>),
    })]
[JsonSerializable(typeof(Platform))]
[JsonSerializable(typeof(PlatformStat))]
[JsonSerializable(typeof(ICollection<PlatformStat>))]
[JsonSerializable(typeof(DockerPlatformDescriptor))]
[JsonSerializable(typeof(DockerSwarmPlatformDescriptor))]
[JsonSerializable(typeof(KubernetesPlatformDescriptor))]
[JsonSerializable(typeof(ICollection<SwarmPeer>))]
[JsonSerializable(typeof(PlatformDescriptor))]
public partial class PlatformJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<RegistryType>),
        typeof(JsonStringEnumConverter<RegistryStatus>),
        typeof(JsonStringEnumConverter<GhcrAccountType>)
    })]
[JsonSerializable(typeof(Registry))]
[JsonSerializable(typeof(AWSRegistry))]
[JsonSerializable(typeof(AzureRegistry))]
[JsonSerializable(typeof(GitlabRegistry))]
[JsonSerializable(typeof(CustomRegistry))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(GitHubRegistry))]
[JsonSerializable(typeof(RegistryConfiguration))]
public partial class RegistryJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(IDictionary<string, IReadOnlyList<HostPortBinding>>))]
public partial class ContainerPortsContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(IEnumerable<string>))]
public partial class ImagTagsContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<StopSignal>),
        typeof(JsonStringEnumConverter<ImageSource>),
        typeof(JsonStringEnumConverter<UpdateBehavior>),
        typeof(JsonStringEnumConverter<AutoUpdateStatus>),
        typeof(JsonStringEnumConverter<ResourceControlState>),
        typeof(JsonStringEnumConverter<ContainerRestartPolicy>),
    })]
[JsonSerializable(typeof(IEnumerable<Guid>))]
[JsonSerializable(typeof(Deployment))]
[JsonSerializable(typeof(DeploymentSpec))]
[JsonSerializable(typeof(HealthCheckConfig))]
public partial class DeploymentJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ActivityEventType>),
    })]
[JsonSerializable(typeof(ActivityEventInfo))]
[JsonSerializable(typeof(DeploymentCreated))]
[JsonSerializable(typeof(DeploymentUpdated))]
[JsonSerializable(typeof(DeploymentDeleted))]
[JsonSerializable(typeof(DeploymentRenamed))]
[JsonSerializable(typeof(DeploymentStarted))]
[JsonSerializable(typeof(DeploymentStopped))]
[JsonSerializable(typeof(DeploymentPaused))]
[JsonSerializable(typeof(DeploymentDegraded))]
[JsonSerializable(typeof(DeploymentApplied))]
[JsonSerializable(typeof(AlertRuleCreated))]
[JsonSerializable(typeof(AlertRuleUpdated))]
[JsonSerializable(typeof(AlertRuleDeleted))]
[JsonSerializable(typeof(RegistryRenamed))]
[JsonSerializable(typeof(RegistryDeleted))]
[JsonSerializable(typeof(RegistryUpdated))]
[JsonSerializable(typeof(RegistryCreated))]
[JsonSerializable(typeof(GitRepoCreated))]
[JsonSerializable(typeof(GitRepoUpdated))]
[JsonSerializable(typeof(GitRepoRenamed))]
[JsonSerializable(typeof(GitRepoDeleted))]
[JsonSerializable(typeof(GitRepoCloned))]
[JsonSerializable(typeof(GitRepoPulled))]

public partial class EventInfoJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<AlertType>),
        typeof(JsonStringEnumConverter<AlertSeverity>),
        typeof(JsonStringEnumConverter<AlertRuleStatus>),
        typeof(JsonStringEnumConverter<AlertResourceType>),
    })]
[JsonSerializable(typeof(AlertEvent))]
[JsonSerializable(typeof(AlertEventInfo))]
public partial class AlertEventJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<AlertType>),
        typeof(JsonStringEnumConverter<DayOfWeek>),
        typeof(JsonStringEnumConverter<ScheduleType>),
        typeof(JsonStringEnumConverter<AlertSeverity>),
        typeof(JsonStringEnumConverter<AlertRuleStatus>),
        typeof(JsonStringEnumConverter<AlertResourceType>),
        typeof(JsonStringEnumConverter<AlertDestination>),
    })]
[JsonSerializable(typeof(AlertRule))]
[JsonSerializable(typeof(AlertEvent))]
[JsonSerializable(typeof(AlertChannel))]
[JsonSerializable(typeof(DailyQuietHour))]
[JsonSerializable(typeof(WeeklyQuietHour))]
[JsonSerializable(typeof(AlertRuleQuietHour))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleLimitedTo>))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleLimitedTo>))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleQuietHour>))]
public partial class AlertRuleJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<GitTransport>),
        typeof(JsonStringEnumConverter<GitAuthType>),
        typeof(JsonStringEnumConverter<GitReposStatus>)
    })]
[JsonSerializable(typeof(GitAccount))]
[JsonSerializable(typeof(BasicAuth))]
[JsonSerializable(typeof(TokenAuth))]
[JsonSerializable(typeof(SshKeyAuth))]
[JsonSerializable(typeof(GitAuthConfiguration))]
[JsonSerializable(typeof(GitRepository))]
[JsonSerializable(typeof(RepoCommand))]
public partial class GitJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<StackSource>),
        typeof(JsonStringEnumConverter<StackReleaseStatus>),
        typeof(JsonStringEnumConverter<StackUpdateBehavior>),
        typeof(JsonStringEnumConverter<ResourceControlState>)
    })]
[JsonSerializable(typeof(IEnumerable<Guid>))]
[JsonSerializable(typeof(Stack))]
[JsonSerializable(typeof(StackPatchModel))]
[JsonSerializable(typeof(StackRelease))]
[JsonSerializable(typeof(StackSpec))]
[JsonSerializable(typeof(ManualStack))]
[JsonSerializable(typeof(GitStack))]
[JsonSerializable(typeof(StackUpdateState))]
[JsonSerializable(typeof(ManualStackUpdateState))]
[JsonSerializable(typeof(GitStackUpdateState))]
[JsonSerializable(typeof(RecreateStackOnNewImageState))]
[JsonSerializable(typeof(RecreateStackOnNewCommitState))]
[JsonSerializable(typeof(ImageUpdateState))]
[JsonSerializable(typeof(IReadOnlyList<ImageUpdateState>))]
public partial class StackJsonContext : JsonSerializerContext
{
}


[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ResourceType>),
        typeof(JsonStringEnumConverter<ResourceAction>)
    })]
[JsonSerializable(typeof(PatchRolePermissionsModel))]
[JsonSerializable(typeof(PatchPermissionModel))]
[JsonSerializable(typeof(PatchUserModel))]
[JsonSerializable(typeof(PatchTeamModel))]
public partial class RoleJsonContext : JsonSerializerContext
{
}