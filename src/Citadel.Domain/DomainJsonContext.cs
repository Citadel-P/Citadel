using Domain.Contracts.Resources.Role;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Automation;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Domain.Entities.Tags;
using Hosting.Common;
using System.Text.Json.Serialization;
using Domain.Contracts.Resources.ResourceBindings;

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
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>),
        typeof(JsonStringEnumConverter<SecretProviderType>),
        typeof(JsonStringEnumConverter<WebhookProvider>),
        typeof(JsonStringEnumConverter<WebhookAuthScheme>),
    })]
[JsonSerializable(typeof(IEnumerable<Guid>))]
[JsonSerializable(typeof(Deployment))]
[JsonSerializable(typeof(DeploymentSpec))]
[JsonSerializable(typeof(Domain.Contracts.Resources.Deployments.DeploymentResultSnapshot))]
[JsonSerializable(typeof(ResourceBindingSnapshot))]
[JsonSerializable(typeof(IReadOnlyList<ResourceBindingSnapshot>))]
[JsonSerializable(typeof(HealthCheckConfig))]
public partial class DeploymentJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>),
        typeof(JsonStringEnumConverter<SecretProviderType>)
    })]
[JsonSerializable(typeof(ResourceBinding))]
[JsonSerializable(typeof(IEnumerable<ResourceBinding>))]
[JsonSerializable(typeof(ResourceBindingSnapshot))]
[JsonSerializable(typeof(IReadOnlyList<ResourceBindingSnapshot>))]
[JsonSerializable(typeof(SecretDefinition))]
[JsonSerializable(typeof(InternalSecretValue))]
[JsonSerializable(typeof(SecretProvider))]
[JsonSerializable(typeof(VaultKvV2SecretProviderConfiguration))]
public partial class ConfigurationJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ActivityEventType>),
        typeof(JsonStringEnumConverter<ActionRunTrigger>),
        typeof(JsonStringEnumConverter<ActionRunStatus>),
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>),
        typeof(JsonStringEnumConverter<SecretProviderType>),
    })]
[JsonSerializable(typeof(ActivityEventInfo))]
[JsonSerializable(typeof(ActivitySourceResource))]
[JsonSerializable(typeof(DeploymentCreated))]
[JsonSerializable(typeof(DeploymentDuplicated))]
[JsonSerializable(typeof(DeploymentUpdated))]
[JsonSerializable(typeof(DeploymentDeleted))]
[JsonSerializable(typeof(DeploymentRenamed))]
[JsonSerializable(typeof(DeploymentStarted))]
[JsonSerializable(typeof(DeploymentStopped))]
[JsonSerializable(typeof(DeploymentPaused))]
[JsonSerializable(typeof(DeploymentDegraded))]
[JsonSerializable(typeof(DeploymentApplied))]
[JsonSerializable(typeof(StackCreated))]
[JsonSerializable(typeof(StackDuplicated))]
[JsonSerializable(typeof(StackUpdated))]
[JsonSerializable(typeof(StackDeleted))]
[JsonSerializable(typeof(StackRenamed))]
[JsonSerializable(typeof(StackStarted))]
[JsonSerializable(typeof(StackStopped))]
[JsonSerializable(typeof(StackPaused))]
[JsonSerializable(typeof(StackDegraded))]
[JsonSerializable(typeof(StackDriftDetected))]
[JsonSerializable(typeof(StackDriftResolved))]
[JsonSerializable(typeof(StackReconciliationAttempted))]
[JsonSerializable(typeof(StackGitUpdateAvailable))]
[JsonSerializable(typeof(StackGitAutoUpdated))]
[JsonSerializable(typeof(StackGitAutoDeployFailed))]
[JsonSerializable(typeof(StackWebhookReceived))]
[JsonSerializable(typeof(StackApplied))]
[JsonSerializable(typeof(StackRollback))]
[JsonSerializable(typeof(Domain.Contracts.Resources.Deployments.DeploymentResultSnapshot))]
[JsonSerializable(typeof(Domain.Contracts.Resources.Stacks.StackResultSnapshot))]
[JsonSerializable(typeof(ResourceBindingSnapshot))]
[JsonSerializable(typeof(IReadOnlyList<ResourceBindingSnapshot>))]
[JsonSerializable(typeof(AlertRuleCreated))]
[JsonSerializable(typeof(AlertRuleUpdated))]
[JsonSerializable(typeof(AlertRuleDeleted))]
[JsonSerializable(typeof(PlatformSnapshot))]
[JsonSerializable(typeof(PlatformCreated))]
[JsonSerializable(typeof(PlatformDeleted))]
[JsonSerializable(typeof(PlatformConnected))]
[JsonSerializable(typeof(PlatformDisconnected))]
[JsonSerializable(typeof(PlatformRenamed))]
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
[JsonSerializable(typeof(GitRepoWebhookReceived))]
[JsonSerializable(typeof(OidcProviderActivitySnapshot))]
[JsonSerializable(typeof(OidcProviderCreated))]
[JsonSerializable(typeof(OidcProviderUpdated))]
[JsonSerializable(typeof(OidcProviderRenamed))]
[JsonSerializable(typeof(OidcProviderDeleted))]
[JsonSerializable(typeof(AutomationActionSnapshot))]
[JsonSerializable(typeof(AutomationActionCreated))]
[JsonSerializable(typeof(AutomationActionUpdated))]
[JsonSerializable(typeof(AutomationActionRenamed))]
[JsonSerializable(typeof(AutomationActionDeleted))]
[JsonSerializable(typeof(AutomationActionRunQueued))]
[JsonSerializable(typeof(AutomationActionRunStarted))]
[JsonSerializable(typeof(AutomationActionRunSucceeded))]
[JsonSerializable(typeof(AutomationActionRunFailed))]
[JsonSerializable(typeof(AutomationActionRunTimedOut))]
[JsonSerializable(typeof(AutomationActionRunCancelled))]
[JsonSerializable(typeof(AutomationActionRunRejected))]

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
[JsonSerializable(typeof(StackDriftDetectedAlertInfo))]
[JsonSerializable(typeof(StackDriftAutoReconciledAlertInfo))]
[JsonSerializable(typeof(StackImageUpdateItem))]
[JsonSerializable(typeof(IReadOnlyList<StackImageUpdateItem>))]
[JsonSerializable(typeof(StackServiceAutoUpdatedAlertInfo))]
[JsonSerializable(typeof(StackServiceAutoDeployFailedAlertInfo))]
[JsonSerializable(typeof(StackGitUpdateAvailableAlertInfo))]
[JsonSerializable(typeof(StackGitAutoUpdatedAlertInfo))]
[JsonSerializable(typeof(StackGitAutoDeployFailedAlertInfo))]
[JsonSerializable(typeof(StackConfigurationResolutionFailedAlertInfo))]
[JsonSerializable(typeof(DeploymentConfigurationResolutionFailedAlertInfo))]
[JsonSerializable(typeof(WebhookAuthenticationFailedAlertInfo))]
[JsonSerializable(typeof(WebhookDispatchFailedAlertInfo))]
[JsonSerializable(typeof(WebhookGitRepoSyncFailedAlertInfo))]
[JsonSerializable(typeof(WebhookStackGitDeployFailedAlertInfo))]
[JsonSerializable(typeof(AutomationActionRunFailedAlertInfo))]
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
        typeof(JsonStringEnumConverter<GitReposStatus>),
        typeof(JsonStringEnumConverter<GitRepositorySyncMode>),
        typeof(JsonStringEnumConverter<WebhookProvider>),
        typeof(JsonStringEnumConverter<WebhookAuthScheme>),
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>)
    })]
[JsonSerializable(typeof(GitAccount))]
[JsonSerializable(typeof(BasicAuth))]
[JsonSerializable(typeof(TokenAuth))]
[JsonSerializable(typeof(SshKeyAuth))]
[JsonSerializable(typeof(GitAuthConfiguration))]
[JsonSerializable(typeof(GitRepository))]
[JsonSerializable(typeof(RepoCommand))]
[JsonSerializable(typeof(RepoWebhookConfig))]
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
        typeof(JsonStringEnumConverter<StackDriftMode>),
        typeof(JsonStringEnumConverter<StackReconciliationStatus>),
        typeof(JsonStringEnumConverter<StackReconciliationActionType>),
        typeof(JsonStringEnumConverter<StackUpdateBehavior>),
        typeof(JsonStringEnumConverter<ResourceControlState>),
        typeof(JsonStringEnumConverter<WebhookProvider>),
        typeof(JsonStringEnumConverter<WebhookAuthScheme>),
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>),
        typeof(JsonStringEnumConverter<SecretProviderType>)
    })]
[JsonSerializable(typeof(IEnumerable<Guid>))]
[JsonSerializable(typeof(Stack))]
[JsonSerializable(typeof(StackPatchModel))]
[JsonSerializable(typeof(StackDriftPolicy))]
[JsonSerializable(typeof(StackDriftReport))]
[JsonSerializable(typeof(StackDrift))]
[JsonSerializable(typeof(MissingContainer))]
[JsonSerializable(typeof(ExtraContainer))]
[JsonSerializable(typeof(ContainerStopped))]
[JsonSerializable(typeof(ContainerPaused))]
[JsonSerializable(typeof(ContainerUnhealthy))]
[JsonSerializable(typeof(ImageMismatch))]
[JsonSerializable(typeof(ConfigHashMismatch))]
[JsonSerializable(typeof(StackReconciliationResult))]
[JsonSerializable(typeof(StackReconciliationAction))]
[JsonSerializable(typeof(StackRelease))]
[JsonSerializable(typeof(StackReleaseSource))]
[JsonSerializable(typeof(Domain.Contracts.Resources.Stacks.StackResultSnapshot))]
[JsonSerializable(typeof(Domain.Contracts.Resources.ResourceBindings.ResourceBindingSnapshot))]
[JsonSerializable(typeof(IReadOnlyList<ResourceBindingSnapshot>))]
[JsonSerializable(typeof(StackSpec))]
[JsonSerializable(typeof(ManualStack))]
[JsonSerializable(typeof(GitStack))]
[JsonSerializable(typeof(WebhookConfig))]
[JsonSerializable(typeof(StackWebhookConfig))]
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
        typeof(JsonStringEnumConverter<WebhookProvider>),
        typeof(JsonStringEnumConverter<WebhookAuthScheme>)
    })]
[JsonSerializable(typeof(AutomationWebhookConfig))]
public partial class AutomationJsonContext : JsonSerializerContext
{
}


[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ResourceType>),
        typeof(JsonStringEnumConverter<PermissionLevel>),
        typeof(JsonStringEnumConverter<SpecificPermission>)
    })]
[JsonSerializable(typeof(PatchRolePermissionsModel))]
[JsonSerializable(typeof(PatchPermissionModel))]
[JsonSerializable(typeof(ResourceAccessView))]
[JsonSerializable(typeof(IEnumerable<ResourceAccessView>))]
[JsonSerializable(typeof(ResourceInfo))]
[JsonSerializable(typeof(IEnumerable<ResourceInfo>))]
[JsonSerializable(typeof(TeamResourceAccessModel))]
[JsonSerializable(typeof(IEnumerable<TeamResourceAccessModel>))]
[JsonSerializable(typeof(IEnumerable<SpecificPermission>))]
[JsonSerializable(typeof(PatchUserModel))]
[JsonSerializable(typeof(PatchTeamModel))]
public partial class RoleJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(IReadOnlyList<TagSummary>))]
public partial class TagJsonContext : JsonSerializerContext
{
}
