using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Platforms;
using Nerdbank.MessagePack;
using PolyType;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Stacks;

namespace WebApi.Hubs;

[GenerateShapeFor<ContainersView>]
[GenerateShapeFor<ContainerView>]
[GenerateShapeFor<List<ContainerView>>]
[GenerateShapeFor<List<ContainerStatView>>]
[GenerateShapeFor<ContainerStatView>]
[GenerateShapeFor<Dictionary<string, string>>]
[GenerateShapeFor<IEnumerable<PlatformView>>]
[GenerateShapeFor<PlatformView>]
[GenerateShapeFor<KubernetesPlatformDescriptor>]
[GenerateShapeFor<DockerSwarmPlatformDescriptor>]
[GenerateShapeFor<DockerPlatformDescriptor>]
[GenerateShapeFor<List<PlatformStatView>>]
[GenerateShapeFor<SwarmInfoView>]
[GenerateShapeFor<List<SwarmPeerView>>]
[GenerateShapeFor<PlatformsView>]
[GenerateShapeFor<PlatformStatsBatchView>]
[GenerateShapeFor<PlatformStatView>]
[GenerateShapeFor<DockerContainer>]
[GenerateShapeFor<List<DockerContainer>>]
[GenerateShapeFor<IEnumerable<DockerContainer>>]
[GenerateShapeFor<DockerContainerStat>]
[GenerateShapeFor<byte[]>]
[GenerateShapeFor<ReadOnlyMemory<byte>>]
[GenerateShapeFor<IDictionary<string, IReadOnlyList<HostPortBinding>>>]
[GenerateShapeFor<ImagesView>]
[GenerateShapeFor<ImageView>]
[GenerateShapeFor<IEnumerable<ImageView>>]
[GenerateShapeFor<DockerNetworkResult>]
[GenerateShapeFor<DockerVolumeResult>]
[GenerateShapeFor<DeploymentsView>]
[GenerateShapeFor<DeploymentView>]
[GenerateShapeFor<StacksView>]
[GenerateShapeFor<StackView>]
[GenerateShapeFor<ActivityView>]
[GenerateShapeFor<AlertEventView>]
[GenerateShapeFor<List<AlertEventView>>]
[GenerateShapeFor<UnresolvedAlertsCountView>]
[GenerateShapeFor<GitRepositoryView>]
[GenerateShapeFor<LatestActivityView>]
[GenerateShapeFor<ActivitySourceResource>]
[GenerateShapeFor<ActivityChangedField>]
[GenerateShapeFor<DeploymentCreated>]
[GenerateShapeFor<DeploymentDuplicated>]
[GenerateShapeFor<DeploymentUpdated>]
[GenerateShapeFor<DeploymentRenamed>]
[GenerateShapeFor<DeploymentDeleted>]
[GenerateShapeFor<DeploymentStarted>]
[GenerateShapeFor<DeploymentStopped>]
[GenerateShapeFor<DeploymentPaused>]
[GenerateShapeFor<DeploymentApplied>]
[GenerateShapeFor<DeploymentDegraded>]
[GenerateShapeFor<StackCreated>]
[GenerateShapeFor<StackDuplicated>]
[GenerateShapeFor<StackUpdated>]
[GenerateShapeFor<StackRenamed>]
[GenerateShapeFor<StackDeleted>]
[GenerateShapeFor<StackStarted>]
[GenerateShapeFor<StackStopped>]
[GenerateShapeFor<StackPaused>]
[GenerateShapeFor<StackApplied>]
[GenerateShapeFor<StackRollback>]
[GenerateShapeFor<StackDegraded>]
[GenerateShapeFor<StackDriftDetected>]
[GenerateShapeFor<StackDriftResolved>]
[GenerateShapeFor<StackReconciliationAttempted>]
[GenerateShapeFor<StackGitUpdateAvailable>]
[GenerateShapeFor<StackGitAutoUpdated>]
[GenerateShapeFor<StackGitAutoDeployFailed>]
[GenerateShapeFor<StackWebhookReceived>]
[GenerateShapeFor<AlertRuleCreated>]
[GenerateShapeFor<AlertRuleUpdated>]
[GenerateShapeFor<AlertRuleDeleted>]
[GenerateShapeFor<AlertRuleRenamed>]
[GenerateShapeFor<PlatformSnapshot>]
[GenerateShapeFor<PlatformCreated>]
[GenerateShapeFor<PlatformDeleted>]
[GenerateShapeFor<PlatformConnected>]
[GenerateShapeFor<PlatformDisconnected>]
[GenerateShapeFor<PlatformRenamed>]
[GenerateShapeFor<RegistryRenamed>]
[GenerateShapeFor<RegistryCreated>]
[GenerateShapeFor<RegistryUpdated>]
[GenerateShapeFor<RegistryDeleted>]
[GenerateShapeFor<GitRepoCreated>]
[GenerateShapeFor<GitRepoUpdated>]
[GenerateShapeFor<GitRepoRenamed>]
[GenerateShapeFor<GitRepoDeleted>]
[GenerateShapeFor<GitRepoCloned>]
[GenerateShapeFor<GitRepoPulled>]
[GenerateShapeFor<GitRepoWebhookReceived>]
[GenerateShapeFor<OidcProviderActivitySnapshot>]
[GenerateShapeFor<OidcProviderCreated>]
[GenerateShapeFor<OidcProviderUpdated>]
[GenerateShapeFor<OidcProviderRenamed>]
[GenerateShapeFor<OidcProviderDeleted>]
[GenerateShapeFor<AutomationActionSnapshot>]
[GenerateShapeFor<AutomationActionCreated>]
[GenerateShapeFor<AutomationActionUpdated>]
[GenerateShapeFor<AutomationActionRenamed>]
[GenerateShapeFor<AutomationActionDeleted>]
[GenerateShapeFor<AutomationActionRunQueued>]
[GenerateShapeFor<AutomationActionRunStarted>]
[GenerateShapeFor<AutomationActionRunSucceeded>]
[GenerateShapeFor<AutomationActionRunFailed>]
[GenerateShapeFor<AutomationActionRunTimedOut>]
[GenerateShapeFor<AutomationActionRunCancelled>]
[GenerateShapeFor<AutomationActionRunRejected>]
[GenerateShapeFor<UserProfileUpdated>]
[GenerateShapeFor<UserPreferencesUpdated>]
[GenerateShapeFor<UserPasswordChanged>]
[GenerateShapeFor<UserSessionRevoked>]
[GenerateShapeFor<UserOtherSessionsRevoked>]
[GenerateShapeFor<WebhookAuthenticationFailedAlertInfo>]
[GenerateShapeFor<WebhookDispatchFailedAlertInfo>]
[GenerateShapeFor<WebhookGitRepoSyncFailedAlertInfo>]
[GenerateShapeFor<WebhookStackGitDeployFailedAlertInfo>]
[GenerateShapeFor<LicenseEnteredGracePeriodAlertInfo>]
[GenerateShapeFor<LicenseExpiredAlertInfo>]
[GenerateShapeFor<TargetResource>]
[GenerateShapeFor<ContainerDataView>]
[GenerateShapeFor<ContainersDataView>]
[GenerateShapeFor<List<ContainerDataView>>]
partial class SignalRMessagePackContext;

internal static class DerivedTypesMapping
{
    internal static DerivedTypeMapping<PlatformDescriptor> PlatformDescriptorMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(PlatformType.Docker)] = typeof(DockerPlatformDescriptor),
        [nameof(PlatformType.Kubernetes)] = typeof(KubernetesPlatformDescriptor),
        [nameof(PlatformType.DockerSwarm)] = typeof(DockerSwarmPlatformDescriptor),
    };

    internal static DerivedTypeMapping<ActivityEventInfo> ActivityEventInfoMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(ActivityEventType.StackCreated)] = typeof(StackCreated),
        [nameof(ActivityEventType.StackDuplicated)] = typeof(StackDuplicated),
        [nameof(ActivityEventType.StackUpdated)] = typeof(StackUpdated),
        [nameof(ActivityEventType.StackRenamed)] = typeof(StackRenamed),
        [nameof(ActivityEventType.StackDeleted)] = typeof(StackDeleted),
        [nameof(ActivityEventType.StackStarted)] = typeof(StackStarted),
        [nameof(ActivityEventType.StackStopped)] = typeof(StackStopped),
        [nameof(ActivityEventType.StackPaused)] = typeof(StackPaused),
        [nameof(ActivityEventType.StackApplied)] = typeof(StackApplied),
        [nameof(ActivityEventType.StackRollback)] = typeof(StackRollback),
        [nameof(ActivityEventType.StackDegraded)] = typeof(StackDegraded),
        [nameof(ActivityEventType.StackDriftDetected)] = typeof(StackDriftDetected),
        [nameof(ActivityEventType.StackDriftResolved)] = typeof(StackDriftResolved),
        [nameof(ActivityEventType.StackReconciliationAttempted)] = typeof(StackReconciliationAttempted),
        [nameof(ActivityEventType.StackGitUpdateAvailable)] = typeof(StackGitUpdateAvailable),
        [nameof(ActivityEventType.StackGitAutoUpdated)] = typeof(StackGitAutoUpdated),
        [nameof(ActivityEventType.StackGitAutoDeployFailed)] = typeof(StackGitAutoDeployFailed),
        [nameof(ActivityEventType.StackWebhookReceived)] = typeof(StackWebhookReceived),
        [nameof(ActivityEventType.DeploymentCreated)] = typeof(DeploymentCreated),
        [nameof(ActivityEventType.DeploymentDuplicated)] = typeof(DeploymentDuplicated),
        [nameof(ActivityEventType.DeploymentUpdated)] = typeof(DeploymentUpdated),
        [nameof(ActivityEventType.DeploymentRenamed)] = typeof(DeploymentRenamed),
        [nameof(ActivityEventType.DeploymentDeleted)] = typeof(DeploymentDeleted),
        [nameof(ActivityEventType.DeploymentStarted)] = typeof(DeploymentStarted),
        [nameof(ActivityEventType.DeploymentStopped)] = typeof(DeploymentStopped),
        [nameof(ActivityEventType.DeploymentPaused)] = typeof(DeploymentPaused),
        [nameof(ActivityEventType.DeploymentApplied)] = typeof(DeploymentApplied),
        [nameof(ActivityEventType.DeploymentDegraded)] = typeof(DeploymentDegraded),
        [nameof(ActivityEventType.AlertRuleCreated)] = typeof(AlertRuleCreated),
        [nameof(ActivityEventType.AlertRuleUpdated)] = typeof(AlertRuleUpdated),
        [nameof(ActivityEventType.AlertRuleDeleted)] = typeof(AlertRuleDeleted),
        [nameof(ActivityEventType.AlertRuleRenamed)] = typeof(AlertRuleRenamed),
        [nameof(ActivityEventType.PlatformCreated)] = typeof(PlatformCreated),
        [nameof(ActivityEventType.PlatformDeleted)] = typeof(PlatformDeleted),
        [nameof(ActivityEventType.PlatformConnected)] = typeof(PlatformConnected),
        [nameof(ActivityEventType.PlatformDisconnected)] = typeof(PlatformDisconnected),
        [nameof(ActivityEventType.PlatformRenamed)] = typeof(PlatformRenamed),
        [nameof(ActivityEventType.RegistryRenamed)] = typeof(RegistryRenamed),
        [nameof(ActivityEventType.RegistryCreated)] = typeof(RegistryCreated),
        [nameof(ActivityEventType.RegistryUpdated)] = typeof(RegistryUpdated),
        [nameof(ActivityEventType.RegistryDeleted)] = typeof(RegistryDeleted),
        [nameof(ActivityEventType.GitRepoCreated)] = typeof(GitRepoCreated),
        [nameof(ActivityEventType.GitRepoUpdated)] = typeof(GitRepoUpdated),
        [nameof(ActivityEventType.GitRepoRenamed)] = typeof(GitRepoRenamed),
        [nameof(ActivityEventType.GitRepoDeleted)] = typeof(GitRepoDeleted),
        [nameof(ActivityEventType.GitRepoCloned)] = typeof(GitRepoCloned),
        [nameof(ActivityEventType.GitRepoPulled)] = typeof(GitRepoPulled),
        [nameof(ActivityEventType.GitRepoWebhookReceived)] = typeof(GitRepoWebhookReceived),
        [nameof(ActivityEventType.OidcProviderCreated)] = typeof(OidcProviderCreated),
        [nameof(ActivityEventType.OidcProviderUpdated)] = typeof(OidcProviderUpdated),
        [nameof(ActivityEventType.OidcProviderRenamed)] = typeof(OidcProviderRenamed),
        [nameof(ActivityEventType.OidcProviderDeleted)] = typeof(OidcProviderDeleted),
        [nameof(ActivityEventType.ActionCreated)] = typeof(AutomationActionCreated),
        [nameof(ActivityEventType.ActionUpdated)] = typeof(AutomationActionUpdated),
        [nameof(ActivityEventType.ActionRenamed)] = typeof(AutomationActionRenamed),
        [nameof(ActivityEventType.ActionDeleted)] = typeof(AutomationActionDeleted),
        [nameof(ActivityEventType.ActionRunQueued)] = typeof(AutomationActionRunQueued),
        [nameof(ActivityEventType.ActionRunStarted)] = typeof(AutomationActionRunStarted),
        [nameof(ActivityEventType.ActionRunSucceeded)] = typeof(AutomationActionRunSucceeded),
        [nameof(ActivityEventType.ActionRunFailed)] = typeof(AutomationActionRunFailed),
        [nameof(ActivityEventType.ActionRunTimedOut)] = typeof(AutomationActionRunTimedOut),
        [nameof(ActivityEventType.ActionRunCancelled)] = typeof(AutomationActionRunCancelled),
        [nameof(ActivityEventType.ActionRunRejected)] = typeof(AutomationActionRunRejected),
        [nameof(ActivityEventType.UserProfileUpdated)] = typeof(UserProfileUpdated),
        [nameof(ActivityEventType.UserPreferencesUpdated)] = typeof(UserPreferencesUpdated),
        [nameof(ActivityEventType.UserPasswordChanged)] = typeof(UserPasswordChanged),
        [nameof(ActivityEventType.UserSessionRevoked)] = typeof(UserSessionRevoked),
        [nameof(ActivityEventType.UserOtherSessionsRevoked)] = typeof(UserOtherSessionsRevoked),
    };
}
