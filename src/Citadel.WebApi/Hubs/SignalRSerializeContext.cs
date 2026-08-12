using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
using Nerdbank.MessagePack;
using PolyType;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Automation;
using WebApi.Routes.Endpoints.Resources.Backups;
using WebApi.Routes.Endpoints.Resources.Builds;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Stacks;
using WebApi.Routes.Endpoints.Resources.Swarm;
using ManagedSwarmServiceView = WebApi.Routes.Endpoints.Resources.SwarmServices.ManagedSwarmServiceView;

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
[GenerateShapeFor<SwarmNodeView>]
[GenerateShapeFor<SwarmNodesView>]
[GenerateShapeFor<IReadOnlyList<SwarmNodeView>>]
[GenerateShapeFor<SwarmServiceView>]
[GenerateShapeFor<SwarmServicesView>]
[GenerateShapeFor<IReadOnlyList<SwarmServiceView>>]
[GenerateShapeFor<SwarmTaskView>]
[GenerateShapeFor<SwarmTasksView>]
[GenerateShapeFor<IReadOnlyList<SwarmTaskView>>]
[GenerateShapeFor<SwarmNetworkView>]
[GenerateShapeFor<SwarmNetworksView>]
[GenerateShapeFor<IReadOnlyList<SwarmNetworkView>>]
[GenerateShapeFor<SwarmSecretView>]
[GenerateShapeFor<SwarmSecretsView>]
[GenerateShapeFor<IReadOnlyList<SwarmSecretView>>]
[GenerateShapeFor<SwarmConfigView>]
[GenerateShapeFor<SwarmConfigsView>]
[GenerateShapeFor<IReadOnlyList<SwarmConfigView>>]
[GenerateShapeFor<SwarmInventoryView>]
[GenerateShapeFor<ManagedSwarmServiceView>]
[GenerateShapeFor<SwarmServiceSpec>]
[GenerateShapeFor<SwarmServiceWebhookConfig>]
[GenerateShapeFor<SwarmServiceImageInfo>]
[GenerateShapeFor<SwarmExternalImage>]
[GenerateShapeFor<SwarmBuildImage>]
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
[GenerateShapeFor<ResourceControlState>]
[GenerateShapeFor<DeploymentSpec>]
[GenerateShapeFor<LocalImage>]
[GenerateShapeFor<ExternalImage>]
[GenerateShapeFor<BuildImage>]
[GenerateShapeFor<StacksView>]
[GenerateShapeFor<StackView>]
[GenerateShapeFor<StackSpec>]
[GenerateShapeFor<ManualStack>]
[GenerateShapeFor<GitStack>]
[GenerateShapeFor<StackBuildImageBinding>]
[GenerateShapeFor<IReadOnlyList<StackBuildImageBinding>>]
[GenerateShapeFor<ActivityView>]
[GenerateShapeFor<ActivityEventInfo>]
[GenerateShapeFor<AlertEventView>]
[GenerateShapeFor<List<AlertEventView>>]
[GenerateShapeFor<PlatformDiskHighAlertInfo>]
[GenerateShapeFor<UnresolvedAlertsCountView>]
[GenerateShapeFor<GitRepositoryView>]
[GenerateShapeFor<BackupRepositoryView>]
[GenerateShapeFor<BackupPolicyView>]
[GenerateShapeFor<BackupRunView>]
[GenerateShapeFor<BackupRunsView>]
[GenerateShapeFor<BackupRestoreRunView>]
[GenerateShapeFor<BackupRestoreRunsView>]
[GenerateShapeFor<BackupRunItemView>]
[GenerateShapeFor<IReadOnlyList<BackupRunItemView>>]
[GenerateShapeFor<List<BackupRunItemView>>]
[GenerateShapeFor<BackupRunWarning>]
[GenerateShapeFor<IReadOnlyList<BackupRunWarning>>]
[GenerateShapeFor<List<BackupRunWarning>>]
[GenerateShapeFor<BackupRepositorySpec>]
[GenerateShapeFor<FileSystemBackupRepositorySpec>]
[GenerateShapeFor<S3CompatibleBackupRepositorySpec>]
[GenerateShapeFor<BackupSourceSpec>]
[GenerateShapeFor<DockerVolumeBackupSource>]
[GenerateShapeFor<CitadelSystemBackupSource>]
[GenerateShapeFor<StackBackupSource>]
[GenerateShapeFor<DeploymentBackupSource>]
[GenerateShapeFor<SwarmServiceBackupSource>]
[GenerateShapeFor<BackupWebhookConfig>]
[GenerateShapeFor<BuildProjectView>]
[GenerateShapeFor<BuildProjectsView>]
[GenerateShapeFor<BuildAgentPoolView>]
[GenerateShapeFor<BuildAgentPoolsView>]
[GenerateShapeFor<BuildRunView>]
[GenerateShapeFor<BuildRunsView>]
[GenerateShapeFor<BuildRunLogEntry>]
[GenerateShapeFor<BuildRunLogEntry[]>]
[GenerateShapeFor<IReadOnlyList<BuildRunLogEntry>>]
[GenerateShapeFor<BuildArgSpec>]
[GenerateShapeFor<IReadOnlyList<BuildArgSpec>>]
[GenerateShapeFor<BuildSecretSpec>]
[GenerateShapeFor<IReadOnlyList<BuildSecretSpec>>]
[GenerateShapeFor<BuildPlatformSnapshot>]
[GenerateShapeFor<BuildRegistrySnapshot>]
[GenerateShapeFor<BuildWebhookConfig>]
[GenerateShapeFor<BuildProjectSnapshot>]
[GenerateShapeFor<BuildAgentPoolProviderSpec>]
[GenerateShapeFor<AwsEc2BuildAgentPoolProviderSpec>]
[GenerateShapeFor<SelfManagedVmBuildAgentPoolProviderSpec>]
[GenerateShapeFor<BuildAgentPoolConnectionMode>]
[GenerateShapeFor<BuildAgentPoolSnapshot>]
[GenerateShapeFor<BuildCreated>]
[GenerateShapeFor<BuildUpdated>]
[GenerateShapeFor<BuildRenamed>]
[GenerateShapeFor<BuildDeleted>]
[GenerateShapeFor<BuildRunQueued>]
[GenerateShapeFor<BuildRunStarted>]
[GenerateShapeFor<BuildRunSucceeded>]
[GenerateShapeFor<BuildRunFailed>]
[GenerateShapeFor<BuildRunTimedOut>]
[GenerateShapeFor<BuildRunCancelled>]
[GenerateShapeFor<BuildWebhookReceived>]
[GenerateShapeFor<SwarmServiceWebhookReceived>]
[GenerateShapeFor<BuildAgentPoolCreated>]
[GenerateShapeFor<BuildAgentPoolUpdated>]
[GenerateShapeFor<BuildAgentPoolRenamed>]
[GenerateShapeFor<BuildAgentPoolDeleted>]
[GenerateShapeFor<BuildAgentPoolTested>]
[GenerateShapeFor<BackupPolicyActivitySnapshot>]
[GenerateShapeFor<BackupPolicyCreated>]
[GenerateShapeFor<BackupPolicyUpdated>]
[GenerateShapeFor<BackupPolicyRenamed>]
[GenerateShapeFor<BackupPolicyArchived>]
[GenerateShapeFor<IReadOnlyList<string>>]
[GenerateShapeFor<IReadOnlyDictionary<string, string>>]
[GenerateShapeFor<AutomationActionView>]
[GenerateShapeFor<AutomationActionRunView>]
[GenerateShapeFor<AutomationWebhookConfig>]
[GenerateShapeFor<LatestActivityView>]
[GenerateShapeFor<ActivitySourceResource>]
[GenerateShapeFor<ActivityChangedField>]
[GenerateShapeFor<DeploymentCreated>]
[GenerateShapeFor<DeploymentDuplicated>]
[GenerateShapeFor<DeploymentAdopted>]
[GenerateShapeFor<DeploymentUpdated>]
[GenerateShapeFor<DeploymentRenamed>]
[GenerateShapeFor<DeploymentDeleted>]
[GenerateShapeFor<DeploymentStarted>]
[GenerateShapeFor<DeploymentStopped>]
[GenerateShapeFor<DeploymentPaused>]
[GenerateShapeFor<DeploymentApplied>]
[GenerateShapeFor<DeploymentDegraded>]
[GenerateShapeFor<SwarmServiceActivitySnapshot>]
[GenerateShapeFor<SwarmServiceCreated>]
[GenerateShapeFor<SwarmServiceAdopted>]
[GenerateShapeFor<SwarmServiceDuplicated>]
[GenerateShapeFor<SwarmServiceUpdated>]
[GenerateShapeFor<SwarmServiceRenamed>]
[GenerateShapeFor<SwarmServiceDeleted>]
[GenerateShapeFor<SwarmServiceApplied>]
[GenerateShapeFor<SwarmServiceScaled>]
[GenerateShapeFor<SwarmServiceForceUpdated>]
[GenerateShapeFor<SwarmServiceOperationFailed>]
[GenerateShapeFor<StackCreated>]
[GenerateShapeFor<StackDuplicated>]
[GenerateShapeFor<StackImported>]
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
[GenerateShapeFor<PlatformNodeAgentLifecycle>]
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
[GenerateShapeFor<InitialAdministratorCreated>]
[GenerateShapeFor<UserProfileUpdated>]
[GenerateShapeFor<UserPreferencesUpdated>]
[GenerateShapeFor<UserPasswordChanged>]
[GenerateShapeFor<UserSessionRevoked>]
[GenerateShapeFor<UserOtherSessionsRevoked>]
[GenerateShapeFor<UserMfaEnabled>]
[GenerateShapeFor<UserMfaDisabled>]
[GenerateShapeFor<UserMfaVerificationFailed>]
[GenerateShapeFor<UserMfaRecoveryCodeUsed>]
[GenerateShapeFor<UserMfaRecoveryCodesRegenerated>]
[GenerateShapeFor<UserMfaResetByAdministrator>]
[GenerateShapeFor<VolumeContentDownloaded>]
[GenerateShapeFor<WebhookAuthenticationFailedAlertInfo>]
[GenerateShapeFor<WebhookDispatchFailedAlertInfo>]
[GenerateShapeFor<WebhookGitRepoSyncFailedAlertInfo>]
[GenerateShapeFor<WebhookStackGitDeployFailedAlertInfo>]
[GenerateShapeFor<BuildRunFailedAlertInfo>]
[GenerateShapeFor<SwarmServiceOperationFailedAlertInfo>]
[GenerateShapeFor<LicenseEnteredGracePeriodAlertInfo>]
[GenerateShapeFor<LicenseExpiredAlertInfo>]
[GenerateShapeFor<TargetResource>]
[GenerateShapeFor<ContainerDataView>]
[GenerateShapeFor<ContainersDataView>]
[GenerateShapeFor<List<ContainerDataView>>]
partial class SignalRMessagePackContext;

internal static class DerivedTypesMapping
{
    internal static DerivedTypeMapping<DeploymentImageInfo> DeploymentImageInfoMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(ImageSource.Local)] = typeof(LocalImage),
        [nameof(ImageSource.External)] = typeof(ExternalImage),
        [nameof(ImageSource.Build)] = typeof(BuildImage),
    };

    internal static DerivedTypeMapping<SwarmServiceImageInfo> SwarmServiceImageInfoMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        ["External"] = typeof(SwarmExternalImage),
        ["Build"] = typeof(SwarmBuildImage),
    };

    internal static DerivedTypeMapping<PlatformDescriptor> PlatformDescriptorMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(PlatformType.Docker)] = typeof(DockerPlatformDescriptor),
        [nameof(PlatformType.Kubernetes)] = typeof(KubernetesPlatformDescriptor),
        [nameof(PlatformType.DockerSwarm)] = typeof(DockerSwarmPlatformDescriptor),
    };

    internal static DerivedTypeMapping<BackupRepositorySpec> BackupRepositorySpecMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(BackupRepositoryType.FileSystem)] = typeof(FileSystemBackupRepositorySpec),
        [nameof(BackupRepositoryType.S3Compatible)] = typeof(S3CompatibleBackupRepositorySpec),
    };

    internal static DerivedTypeMapping<BackupSourceSpec> BackupSourceSpecMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(BackupSourceType.DockerVolume)] = typeof(DockerVolumeBackupSource),
        [nameof(BackupSourceType.CitadelSystem)] = typeof(CitadelSystemBackupSource),
        [nameof(BackupSourceType.Stack)] = typeof(StackBackupSource),
        [nameof(BackupSourceType.Deployment)] = typeof(DeploymentBackupSource),
        [nameof(BackupSourceType.SwarmService)] = typeof(SwarmServiceBackupSource),
    };

    internal static DerivedTypeMapping<BuildAgentPoolProviderSpec> BuildAgentPoolProviderSpecMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(BuildAgentPoolProvider.AwsEc2)] = typeof(AwsEc2BuildAgentPoolProviderSpec),
        [nameof(BuildAgentPoolProvider.SelfManagedVm)] = typeof(SelfManagedVmBuildAgentPoolProviderSpec),
    };

    internal static DerivedTypeMapping<ActivityEventInfo> ActivityEventInfoMappings = new(SignalRMessagePackContext.GeneratedTypeShapeProvider)
    {
        [nameof(ActivityEventType.StackCreated)] = typeof(StackCreated),
        [nameof(ActivityEventType.StackDuplicated)] = typeof(StackDuplicated),
        [nameof(ActivityEventType.StackImported)] = typeof(StackImported),
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
        [nameof(ActivityEventType.DeploymentAdopted)] = typeof(DeploymentAdopted),
        [nameof(ActivityEventType.DeploymentUpdated)] = typeof(DeploymentUpdated),
        [nameof(ActivityEventType.DeploymentRenamed)] = typeof(DeploymentRenamed),
        [nameof(ActivityEventType.DeploymentDeleted)] = typeof(DeploymentDeleted),
        [nameof(ActivityEventType.DeploymentStarted)] = typeof(DeploymentStarted),
        [nameof(ActivityEventType.DeploymentStopped)] = typeof(DeploymentStopped),
        [nameof(ActivityEventType.DeploymentPaused)] = typeof(DeploymentPaused),
        [nameof(ActivityEventType.DeploymentApplied)] = typeof(DeploymentApplied),
        [nameof(ActivityEventType.DeploymentDegraded)] = typeof(DeploymentDegraded),
        [nameof(ActivityEventType.SwarmServiceCreated)] = typeof(SwarmServiceCreated),
        [nameof(ActivityEventType.SwarmServiceAdopted)] = typeof(SwarmServiceAdopted),
        [nameof(ActivityEventType.SwarmServiceDuplicated)] = typeof(SwarmServiceDuplicated),
        [nameof(ActivityEventType.SwarmServiceUpdated)] = typeof(SwarmServiceUpdated),
        [nameof(ActivityEventType.SwarmServiceRenamed)] = typeof(SwarmServiceRenamed),
        [nameof(ActivityEventType.SwarmServiceDeleted)] = typeof(SwarmServiceDeleted),
        [nameof(ActivityEventType.SwarmServiceApplied)] = typeof(SwarmServiceApplied),
        [nameof(ActivityEventType.SwarmServiceScaled)] = typeof(SwarmServiceScaled),
        [nameof(ActivityEventType.SwarmServiceForceUpdated)] = typeof(SwarmServiceForceUpdated),
        [nameof(ActivityEventType.SwarmServiceOperationFailed)] = typeof(SwarmServiceOperationFailed),
        [nameof(ActivityEventType.AlertRuleCreated)] = typeof(AlertRuleCreated),
        [nameof(ActivityEventType.AlertRuleUpdated)] = typeof(AlertRuleUpdated),
        [nameof(ActivityEventType.AlertRuleDeleted)] = typeof(AlertRuleDeleted),
        [nameof(ActivityEventType.AlertRuleRenamed)] = typeof(AlertRuleRenamed),
        [nameof(ActivityEventType.PlatformCreated)] = typeof(PlatformCreated),
        [nameof(ActivityEventType.PlatformDeleted)] = typeof(PlatformDeleted),
        [nameof(ActivityEventType.PlatformConnected)] = typeof(PlatformConnected),
        [nameof(ActivityEventType.PlatformDisconnected)] = typeof(PlatformDisconnected),
        [nameof(ActivityEventType.PlatformRenamed)] = typeof(PlatformRenamed),
        [nameof(ActivityEventType.PlatformNodeAgentLifecycle)] = typeof(PlatformNodeAgentLifecycle),
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
        [nameof(ActivityEventType.BuildCreated)] = typeof(BuildCreated),
        [nameof(ActivityEventType.BuildUpdated)] = typeof(BuildUpdated),
        [nameof(ActivityEventType.BuildRenamed)] = typeof(BuildRenamed),
        [nameof(ActivityEventType.BuildDeleted)] = typeof(BuildDeleted),
        [nameof(ActivityEventType.BuildRunQueued)] = typeof(BuildRunQueued),
        [nameof(ActivityEventType.BuildRunStarted)] = typeof(BuildRunStarted),
        [nameof(ActivityEventType.BuildRunSucceeded)] = typeof(BuildRunSucceeded),
        [nameof(ActivityEventType.BuildRunFailed)] = typeof(BuildRunFailed),
        [nameof(ActivityEventType.BuildRunTimedOut)] = typeof(BuildRunTimedOut),
        [nameof(ActivityEventType.BuildRunCancelled)] = typeof(BuildRunCancelled),
        [nameof(ActivityEventType.BuildWebhookReceived)] = typeof(BuildWebhookReceived),
        [nameof(ActivityEventType.SwarmServiceWebhookReceived)] = typeof(SwarmServiceWebhookReceived),
        [nameof(ActivityEventType.BuildAgentPoolCreated)] = typeof(BuildAgentPoolCreated),
        [nameof(ActivityEventType.BuildAgentPoolUpdated)] = typeof(BuildAgentPoolUpdated),
        [nameof(ActivityEventType.BuildAgentPoolRenamed)] = typeof(BuildAgentPoolRenamed),
        [nameof(ActivityEventType.BuildAgentPoolDeleted)] = typeof(BuildAgentPoolDeleted),
        [nameof(ActivityEventType.BuildAgentPoolTested)] = typeof(BuildAgentPoolTested),
        [nameof(ActivityEventType.BackupPolicyCreated)] = typeof(BackupPolicyCreated),
        [nameof(ActivityEventType.BackupPolicyUpdated)] = typeof(BackupPolicyUpdated),
        [nameof(ActivityEventType.BackupPolicyRenamed)] = typeof(BackupPolicyRenamed),
        [nameof(ActivityEventType.BackupPolicyArchived)] = typeof(BackupPolicyArchived),
        [nameof(ActivityEventType.InitialAdministratorCreated)] = typeof(InitialAdministratorCreated),
        [nameof(ActivityEventType.UserProfileUpdated)] = typeof(UserProfileUpdated),
        [nameof(ActivityEventType.UserPreferencesUpdated)] = typeof(UserPreferencesUpdated),
        [nameof(ActivityEventType.UserPasswordChanged)] = typeof(UserPasswordChanged),
        [nameof(ActivityEventType.UserSessionRevoked)] = typeof(UserSessionRevoked),
        [nameof(ActivityEventType.UserOtherSessionsRevoked)] = typeof(UserOtherSessionsRevoked),
        [nameof(ActivityEventType.UserMfaEnabled)] = typeof(UserMfaEnabled),
        [nameof(ActivityEventType.UserMfaDisabled)] = typeof(UserMfaDisabled),
        [nameof(ActivityEventType.UserMfaVerificationFailed)] = typeof(UserMfaVerificationFailed),
        [nameof(ActivityEventType.UserMfaRecoveryCodeUsed)] = typeof(UserMfaRecoveryCodeUsed),
        [nameof(ActivityEventType.UserMfaRecoveryCodesRegenerated)] = typeof(UserMfaRecoveryCodesRegenerated),
        [nameof(ActivityEventType.UserMfaResetByAdministrator)] = typeof(UserMfaResetByAdministrator),
        [nameof(ActivityEventType.VolumeContentDownloaded)] = typeof(VolumeContentDownloaded),
    };
}
