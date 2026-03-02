using System.Text.Json.Serialization;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;

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
public partial class  DeploymentJsonContext: JsonSerializerContext
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
public partial class  EventInfoJsonContext : JsonSerializerContext
{
    
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
    typeof(JsonStringEnumConverter<AlertType>),
    typeof(JsonStringEnumConverter<AlertSeverity>),
    typeof(JsonStringEnumConverter<AlertResourceType>),
    })]
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
    typeof(JsonStringEnumConverter<ScheduleType>),
    typeof(JsonStringEnumConverter<AlertSeverity>),
    typeof(JsonStringEnumConverter<AlertResourceType>),
    typeof(JsonStringEnumConverter<AlertDestination>),
    })]
[JsonSerializable(typeof(AlertRule))]
[JsonSerializable(typeof(AlertChannel))]
[JsonSerializable(typeof(DailyQuietHour))]
[JsonSerializable(typeof(WeeklyQuietHour))]
[JsonSerializable(typeof(AlertRuleQuietHour))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleLimitedTo>))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleLimitedTo>))]
[JsonSerializable(typeof(IReadOnlyCollection<AlertRuleQuietHour>))]
public partial class AlertRuleJsonContext : JsonSerializerContext { }
