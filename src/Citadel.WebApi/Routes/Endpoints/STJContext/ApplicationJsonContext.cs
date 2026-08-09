using Application.Features.Identity.Auth.Models;
using Application.Features.Search.Models;
using Application.Features.Images.Queries;
using Application.Features.ResourceBindings.Models;
using Application.Features.GitRepositories.Queries;
using Application.Features.Stacks.Queries;
using Application.Services.Backups;
using Domain;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Automation;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Compose;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Stacks;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Alerts;
using Domain.Entities.Automation;
using Domain.Entities.Backups;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Licensing;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Microsoft.AspNetCore.Mvc;
using System.Text.Json.Serialization;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Automation;
using WebApi.Routes.Endpoints.Resources.Backups;
using WebApi.Routes.Endpoints.Resources.Builds;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.ResourceBindings;
using WebApi.Routes.Endpoints.Resources.Tags;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitAccounts;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Identity.Actors;
using WebApi.Routes.Endpoints.Resources.Identity.Auth;
using WebApi.Routes.Endpoints.Resources.Identity.Mfa;
using WebApi.Routes.Endpoints.Resources.Identity.Setup;
using WebApi.Routes.Endpoints.Resources.Identity.Profile;
using WebApi.Routes.Endpoints.Resources.Identity.Roles;
using WebApi.Routes.Endpoints.Resources.Identity.Teams;
using WebApi.Routes.Endpoints.Resources.Identity.Users;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Licensing;
using WebApi.Routes.Endpoints.Resources.Lookup;
using WebApi.Routes.Endpoints.Resources.Networks;
using WebApi.Routes.Endpoints.Resources.Oidc;
using WebApi.Routes.Endpoints.Resources.Paging;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.Stacks;
using WebApi.Routes.Endpoints.Resources.Swarm;
using WebApi.Routes.Endpoints.Resources.Search;
using WebApi.Routes.Endpoints.Resources.Volumes;
using Application.Features.Webhooks.Commands;
using Domain.Contracts.Resources.ResourceBindings;
using ApiActorView = WebApi.Routes.Endpoints.Resources.Identity.Actors.ActorView;
using DeleteImageResponseItem = Domain.Contracts.Resources.Images.DeleteImageResponseItem;

namespace Application.Models;

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    PropertyNameCaseInsensitive = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ActorType>),
        typeof(JsonStringEnumConverter<ActionRunTrigger>),
        typeof(JsonStringEnumConverter<ActionRunStatus>),
        typeof(JsonStringEnumConverter<ResourceControlState>),
        typeof(JsonStringEnumConverter<SwarmServiceHealth>),
        typeof(JsonStringEnumConverter<SwarmServiceSynchronizationState>),
        typeof(JsonStringEnumConverter<SwarmServiceOperationKind>),
        typeof(JsonStringEnumConverter<SwarmServiceOperationState>),
        typeof(JsonStringEnumConverter<SwarmServiceSchedulingMode>),
        typeof(JsonStringEnumConverter<SwarmServicePortPublishMode>),
        typeof(JsonStringEnumConverter<SwarmServiceMountKind>),
        typeof(JsonStringEnumConverter<SwarmServiceRestartCondition>),
        typeof(JsonStringEnumConverter<SwarmServiceUpdateOrder>),
        typeof(JsonStringEnumConverter<SwarmServiceUpdateFailureAction>),
        typeof(JsonStringEnumConverter<ResourceBindingKind>),
        typeof(JsonStringEnumConverter<ResourceBindingScope>),
        typeof(JsonStringEnumConverter<SecretDeliveryMode>),
        typeof(JsonStringEnumConverter<SecretProviderType>),
        typeof(JsonStringEnumConverter<WebhookProvider>),
        typeof(JsonStringEnumConverter<WebhookAuthScheme>),
        typeof(JsonStringEnumConverter<WebhookExecution>),
        typeof(JsonStringEnumConverter<UserDateTimeFormat>),
        typeof(JsonStringEnumConverter<UserTheme>),
        typeof(JsonStringEnumConverter<CurrentProfileAuthenticationType>),
        typeof(JsonStringEnumConverter<LoginNextStep>),
        typeof(JsonStringEnumConverter<MfaPolicy>),
        typeof(JsonStringEnumConverter<LicenseCapability>),
        typeof(JsonStringEnumConverter<LicenseStatus>),
        typeof(JsonStringEnumConverter<BackupSourceType>),
        typeof(JsonStringEnumConverter<VolumeBackupConsistency>),
        typeof(JsonStringEnumConverter<BackupRepositoryType>),
        typeof(JsonStringEnumConverter<BackupExecutionLocation>),
        typeof(JsonStringEnumConverter<S3BucketLookup>),
        typeof(JsonStringEnumConverter<BackupRepositoryStatus>),
        typeof(JsonStringEnumConverter<BackupRepositoryValidationStatus>),
        typeof(JsonStringEnumConverter<BackupRunTrigger>),
        typeof(JsonStringEnumConverter<BackupRunStatus>),
        typeof(JsonStringEnumConverter<GlobalSearchResourceType>),
        typeof(JsonStringEnumConverter<GlobalSearchCategory>),
        typeof(JsonStringEnumConverter<SearchStatusTone>),
        typeof(JsonStringEnumConverter<BackupRunItemStatus>),
        typeof(JsonStringEnumConverter<BackupSnapshotAvailability>),
        typeof(JsonStringEnumConverter<BackupCoverageStatus>),
        typeof(JsonStringEnumConverter<BackupCoverageResourceType>),
        typeof(JsonStringEnumConverter<BackupRestoreStatus>),
        typeof(JsonStringEnumConverter<BuildProjectBuilderKind>),
        typeof(JsonStringEnumConverter<BuildAgentPoolProvider>),
        typeof(JsonStringEnumConverter<BuildAgentPoolConnectionMode>),
        typeof(JsonStringEnumConverter<CpuArchitecture>),
        typeof(JsonStringEnumConverter<BuildAgentPoolValidationStatus>),
        typeof(JsonStringEnumConverter<BuildRunTrigger>),
        typeof(JsonStringEnumConverter<BuildRunStatus>),
        typeof(JsonStringEnumConverter<StackImportKind>),
        typeof(JsonStringEnumConverter<VolumeFileEntryType>)
    })]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(Guid[]))]
[JsonSerializable(typeof(LookupResourceType))]
[JsonSerializable(typeof(LookupResourceType?))]
[JsonSerializable(typeof(ResourceType))]
[JsonSerializable(typeof(ResourceType?))]
[JsonSerializable(typeof(UserDateTimeFormat))]
[JsonSerializable(typeof(UserDateTimeFormat?))]
[JsonSerializable(typeof(UserTheme))]
[JsonSerializable(typeof(UserTheme?))]
[JsonSerializable(typeof(CurrentProfileAuthenticationType))]
[JsonSerializable(typeof(LicenseCapability))]
[JsonSerializable(typeof(LicenseCapability?))]
[JsonSerializable(typeof(LicenseStatus))]
[JsonSerializable(typeof(LicenseStatus?))]
[JsonSerializable(typeof(ResourceBindingKind))]
[JsonSerializable(typeof(ResourceBindingKind?))]
[JsonSerializable(typeof(ResourceBindingScope))]
[JsonSerializable(typeof(ResourceBindingScope?))]
[JsonSerializable(typeof(SecretDeliveryMode))]
[JsonSerializable(typeof(SecretDeliveryMode?))]
[JsonSerializable(typeof(SecretProviderType))]
[JsonSerializable(typeof(SecretProviderType?))]
[JsonSerializable(typeof(ResourceBindingSnapshot))]
[JsonSerializable(typeof(IReadOnlyList<ResourceBindingSnapshot>))]
[JsonSerializable(typeof(ActorType))]
[JsonSerializable(typeof(ActorType?))]
[JsonSerializable(typeof(TargetResource))]
[JsonSerializable(typeof(ApiActorView))]
[JsonSerializable(typeof(PatchActorEnabledInput))]
[JsonSerializable(typeof(List<Platform>))]
[JsonSerializable(typeof(List<PlatformStat>))]
[JsonSerializable(typeof(SwarmPeer))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(SwarmNodeView))]
[JsonSerializable(typeof(SwarmNodesView))]
[JsonSerializable(typeof(SwarmNodeInspectView))]
[JsonSerializable(typeof(UpdateSwarmNodeInput))]
[JsonSerializable(typeof(UpdateSwarmNodesAvailabilityInput))]
[JsonSerializable(typeof(SwarmServiceView))]
[JsonSerializable(typeof(SwarmServicesView))]
[JsonSerializable(typeof(SwarmServiceInspectView))]
[JsonSerializable(typeof(SwarmTaskView))]
[JsonSerializable(typeof(SwarmTasksView))]
[JsonSerializable(typeof(SwarmTaskInspectView))]
[JsonSerializable(typeof(SwarmTaskStatsView))]
[JsonSerializable(typeof(SwarmTaskTerminalView))]
[JsonSerializable(typeof(SwarmNetworkView))]
[JsonSerializable(typeof(SwarmNetworksView))]
[JsonSerializable(typeof(SwarmSecretView))]
[JsonSerializable(typeof(SwarmSecretsView))]
[JsonSerializable(typeof(SwarmConfigView))]
[JsonSerializable(typeof(SwarmConfigDataView))]
[JsonSerializable(typeof(SwarmConfigsView))]
[JsonSerializable(typeof(CreateSwarmSecretInput))]
[JsonSerializable(typeof(CreateSwarmConfigInput))]
[JsonSerializable(typeof(UpdateSwarmResourceLabelsInput))]
[JsonSerializable(typeof(DeleteSwarmResourcesInput))]
[JsonSerializable(typeof(SwarmInventoryView))]
[JsonSerializable(typeof(SwarmLogsView))]
[JsonSerializable(typeof(SwarmOverviewView))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(PlatformStatsView))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(List<ContainerView>))]
[JsonSerializable(typeof(IDictionary<string, IReadOnlyList<HostPortBinding>>))]
[JsonSerializable(typeof(EndpointSettingsView))]
[JsonSerializable(typeof(LoginRequest))]
[JsonSerializable(typeof(LoginResponse))]
[JsonSerializable(typeof(LoginNextStep))]
[JsonSerializable(typeof(RefreshTokenResponse))]
[JsonSerializable(typeof(MfaPolicy))]
[JsonSerializable(typeof(MfaVerificationInput))]
[JsonSerializable(typeof(ConfirmMandatoryMfaSetupInput))]
[JsonSerializable(typeof(StartProfileMfaSetupInput))]
[JsonSerializable(typeof(ConfirmProfileMfaSetupInput))]
[JsonSerializable(typeof(DisableProfileMfaInput))]
[JsonSerializable(typeof(RegenerateProfileMfaRecoveryCodesInput))]
[JsonSerializable(typeof(MfaVerificationView))]
[JsonSerializable(typeof(MandatoryMfaSetupView))]
[JsonSerializable(typeof(MandatoryMfaSetupCompleteView))]
[JsonSerializable(typeof(ProfileMfaSetupView))]
[JsonSerializable(typeof(ProfileMfaStatusView))]
[JsonSerializable(typeof(ProfileMfaRecoveryCodesView))]
[JsonSerializable(typeof(SetupStatusView))]
[JsonSerializable(typeof(InitializeCitadelInput))]
[JsonSerializable(typeof(ApplicationInfoView))]
[JsonSerializable(typeof(CurrentProfileView))]
[JsonSerializable(typeof(CurrentProfileAuthenticationView))]
[JsonSerializable(typeof(ProfileResourceInfoView))]
[JsonSerializable(typeof(UpdateCurrentProfileInput))]
[JsonSerializable(typeof(PatchUserPreferencesInput))]
[JsonSerializable(typeof(PatchUserPreferencesInputPatchDocument))]
[JsonSerializable(typeof(UserPreferencesView))]
[JsonSerializable(typeof(ChangeCurrentPasswordInput))]
[JsonSerializable(typeof(UserSessionSummaryView))]
[JsonSerializable(typeof(UserSessionsView))]
[JsonSerializable(typeof(RevokeOtherProfileSessionsView))]
[JsonSerializable(typeof(InstallLicenseInput))]
[JsonSerializable(typeof(LicenseView))]
[JsonSerializable(typeof(LicenseEntitlementsView))]
[JsonSerializable(typeof(LicenseCapabilityView))]
[JsonSerializable(typeof(IReadOnlyList<LicenseCapabilityView>))]
[JsonSerializable(typeof(LicenseRequestView))]
[JsonSerializable(typeof(OidcProviderInput))]
[JsonSerializable(typeof(UpdateOidcProviderInput))]
[JsonSerializable(typeof(UpdateOidcProviderPatchDocument))]
[JsonSerializable(typeof(TestOidcProviderDiscoveryInput))]
[JsonSerializable(typeof(OidcProviderView))]
[JsonSerializable(typeof(OidcProvidersView))]
[JsonSerializable(typeof(OidcLoginProviderView))]
[JsonSerializable(typeof(OidcLoginProvidersView))]
[JsonSerializable(typeof(OidcDiscoveryResultView))]
[JsonSerializable(typeof(AutomationActionInput))]
[JsonSerializable(typeof(UpdateAutomationActionInput))]
[JsonSerializable(typeof(UpdateAutomationActionInputPatchDocument))]
[JsonSerializable(typeof(RunAutomationActionInput))]
[JsonSerializable(typeof(TestAutomationActionInput))]
[JsonSerializable(typeof(AutomationActionView))]
[JsonSerializable(typeof(AutomationActionsView))]
[JsonSerializable(typeof(AutomationActionRunView))]
[JsonSerializable(typeof(AutomationActionRunsView))]
[JsonSerializable(typeof(AutomationActionRunLogsView))]
[JsonSerializable(typeof(BackupSourceSpec))]
[JsonSerializable(typeof(DockerVolumeBackupSource))]
[JsonSerializable(typeof(CitadelSystemBackupSource))]
[JsonSerializable(typeof(StackBackupSource))]
[JsonSerializable(typeof(DeploymentBackupSource))]
[JsonSerializable(typeof(BackupRepositorySpec))]
[JsonSerializable(typeof(FileSystemBackupRepositorySpec))]
[JsonSerializable(typeof(S3CompatibleBackupRepositorySpec))]
[JsonSerializable(typeof(BackupExecutionContext))]
[JsonSerializable(typeof(BackupRunWarning))]
[JsonSerializable(typeof(IReadOnlyList<BackupRunWarning>))]
[JsonSerializable(typeof(BackupAffectedContainer))]
[JsonSerializable(typeof(IReadOnlyList<BackupAffectedContainer>))]
[JsonSerializable(typeof(BackupWebhookConfig))]
[JsonSerializable(typeof(BackupRepositoryInput))]
[JsonSerializable(typeof(UpdateBackupRepositoryInput))]
[JsonSerializable(typeof(UpdateBackupRepositoryInputPatchDocument))]
[JsonSerializable(typeof(ValidateBackupRepositoryInput))]
[JsonSerializable(typeof(BackupRepositoryView))]
[JsonSerializable(typeof(BackupRepositoriesView))]
[JsonSerializable(typeof(BackupRepositoryValidationView))]
[JsonSerializable(typeof(BackupRunStatus))]
[JsonSerializable(typeof(BackupRunStatus?))]
[JsonSerializable(typeof(BackupRunItemStatus))]
[JsonSerializable(typeof(BackupRunItemStatus?))]
[JsonSerializable(typeof(BackupRunStreamItem))]
[JsonSerializable(typeof(IAsyncEnumerable<BackupRunStreamItem>))]
[JsonSerializable(typeof(BackupRestoreRunStreamItem))]
[JsonSerializable(typeof(BackupRestoreRunStreamItem[]))]
[JsonSerializable(typeof(IAsyncEnumerable<BackupRestoreRunStreamItem>))]
[JsonSerializable(typeof(BackupRunItemView))]
[JsonSerializable(typeof(IReadOnlyList<BackupRunItemView>))]
[JsonSerializable(typeof(BackupPolicyInput))]
[JsonSerializable(typeof(UpdateBackupPolicyInput))]
[JsonSerializable(typeof(UpdateBackupPolicyInputPatchDocument))]
[JsonSerializable(typeof(QueueBackupRunInput))]
[JsonSerializable(typeof(RestoreVolumeInput))]
[JsonSerializable(typeof(BackupPolicyView))]
[JsonSerializable(typeof(BackupPoliciesView))]
[JsonSerializable(typeof(PlatformBackupSummaryView))]
[JsonSerializable(typeof(PlatformBackupSummariesView))]
[JsonSerializable(typeof(BackupRunView))]
[JsonSerializable(typeof(BackupRunsView))]
[JsonSerializable(typeof(BackupRestoreRunView))]
[JsonSerializable(typeof(BackupRestoreRunsView))]
[JsonSerializable(typeof(BackupLogsView))]
[JsonSerializable(typeof(BackupEventsView))]
[JsonSerializable(typeof(BackupCoverageView))]
[JsonSerializable(typeof(StackBackupSourcePreviewView))]
[JsonSerializable(typeof(StackBackupVolumeView))]
[JsonSerializable(typeof(DeploymentBackupSourcePreviewView))]
[JsonSerializable(typeof(DeploymentBackupVolumeView))]
[JsonSerializable(typeof(BuildArgSpec))]
[JsonSerializable(typeof(IReadOnlyList<BuildArgSpec>))]
[JsonSerializable(typeof(BuildSecretSpec))]
[JsonSerializable(typeof(IReadOnlyList<BuildSecretSpec>))]
[JsonSerializable(typeof(BuildPlatformSnapshot))]
[JsonSerializable(typeof(BuildRegistrySnapshot))]
[JsonSerializable(typeof(BuildWebhookConfig))]
[JsonSerializable(typeof(BuildRunTrigger))]
[JsonSerializable(typeof(BuildRunStatus))]
[JsonSerializable(typeof(BuildProjectBuilderKind))]
[JsonSerializable(typeof(BuildProjectBuilderKind?))]
[JsonSerializable(typeof(BuildAgentPoolProvider))]
[JsonSerializable(typeof(BuildAgentPoolConnectionMode))]
[JsonSerializable(typeof(BuildAgentPoolProviderSpec))]
[JsonSerializable(typeof(AwsEc2BuildAgentPoolProviderSpec))]
[JsonSerializable(typeof(SelfManagedVmBuildAgentPoolProviderSpec))]
[JsonSerializable(typeof(CpuArchitecture))]
[JsonSerializable(typeof(BuildAgentPoolValidationStatus))]
[JsonSerializable(typeof(BuildProjectInput))]
[JsonSerializable(typeof(UpdateBuildProjectInput))]
[JsonSerializable(typeof(UpdateBuildProjectInputPatchDocument))]
[JsonSerializable(typeof(BuildAgentPoolInput))]
[JsonSerializable(typeof(UpdateBuildAgentPoolInput))]
[JsonSerializable(typeof(UpdateBuildAgentPoolInputPatchDocument))]
[JsonSerializable(typeof(QueueBuildRunInput))]
[JsonSerializable(typeof(BuildProjectView))]
[JsonSerializable(typeof(BuildProjectsView))]
[JsonSerializable(typeof(BuildAgentPoolView))]
[JsonSerializable(typeof(BuildAgentPoolsView))]
[JsonSerializable(typeof(BuildRunView))]
[JsonSerializable(typeof(BuildRunsView))]
[JsonSerializable(typeof(BuildLogsView))]
[JsonSerializable(typeof(StackVolumeKind))]
[JsonSerializable(typeof(PlatformInput))]
[JsonSerializable(typeof(CreatePlatformInput))]
[JsonSerializable(typeof(PrunePlatformInput))]
[JsonSerializable(typeof(PrunePlatformView))]
[JsonSerializable(typeof(PruneResource))]
[JsonSerializable(typeof(PruneResource?))]
[JsonSerializable(typeof(PlatformsView))]
[JsonSerializable(typeof(AgentSetupView))]
[JsonSerializable(typeof(EdgeAgentEnrollmentView))]
[JsonSerializable(typeof(EdgeAgentEnrollmentInstructionsView))]
[JsonSerializable(typeof(EdgeAgentStatusView))]
[JsonSerializable(typeof(PlatformDescriptor))]
[JsonSerializable(typeof(DockerPlatformDescriptor))]
[JsonSerializable(typeof(DockerSwarmPlatformDescriptor))]
[JsonSerializable(typeof(KubernetesPlatformDescriptor))]
[JsonSerializable(typeof(ContainersView))]
[JsonSerializable(typeof(ContainerView))]
[JsonSerializable(typeof(PlatformInputPatchDocument))]
[JsonSerializable(typeof(ContainerStatsView))]
[JsonSerializable(typeof(ContainerStatView))]
[JsonSerializable(typeof(HttpValidationProblemDetails))]
[JsonSerializable(typeof(Dictionary<string, string[]>))]
[JsonSerializable(typeof(ContainerInspectView))]
[JsonSerializable(typeof(NetworkSettingsInfo))]
[JsonSerializable(typeof(ContainerInspectionInfo))]
[JsonSerializable(typeof(ContainerRuntimeState))]
[JsonSerializable(typeof(HostConfiguration))]
[JsonSerializable(typeof(GraphDriverDataInfo))]
[JsonSerializable(typeof(IReadOnlyList<IpAddressInfo>))]
[JsonSerializable(typeof(IReadOnlyList<MountPointInfo>))]
[JsonSerializable(typeof(ContainerConfiguration))]
[JsonSerializable(typeof(RestartPolicy))]
[JsonSerializable(typeof(Ulimit))]
[JsonSerializable(typeof(BindOptions))]
[JsonSerializable(typeof(VolumeOptions))]
[JsonSerializable(typeof(DeleteContainersRequest))]
[JsonSerializable(typeof(RegistryInput))]
[JsonSerializable(typeof(CreateRegistryInput))]
[JsonSerializable(typeof(PatchRegistryInput))]
[JsonSerializable(typeof(RegistryInputPatchDocument))]
[JsonSerializable(typeof(DeleteRegistriesInput))]
[JsonSerializable(typeof(RegistriesView))]
[JsonSerializable(typeof(RegistryView))]
[JsonSerializable(typeof(RegistryConfiguration))]
[JsonSerializable(typeof(GitAccount))]
[JsonSerializable(typeof(GitAccountInput))]
[JsonSerializable(typeof(GitAccountInputPatchDocument))]
[JsonSerializable(typeof(DeleteGitAccountsInput))]
[JsonSerializable(typeof(GitAccountsView))]
[JsonSerializable(typeof(GitAccountView))]
[JsonSerializable(typeof(GitAccountConfigView))]
[JsonSerializable(typeof(GitAuthConfiguration))]
[JsonSerializable(typeof(BasicAuth))]
[JsonSerializable(typeof(TokenAuth))]
[JsonSerializable(typeof(SshKeyAuth))]
[JsonSerializable(typeof(GitRepositorySyncMode))]
[JsonSerializable(typeof(GitRepository))]
[JsonSerializable(typeof(RepoWebhookConfig))]
[JsonSerializable(typeof(CreateGitRepositoryInput))]
[JsonSerializable(typeof(GitRepositoryInputPatchDocument))]
[JsonSerializable(typeof(DeleteGitRepositoriesInput))]
[JsonSerializable(typeof(GitRepositoriesView))]
[JsonSerializable(typeof(GitRepositoryView))]
[JsonSerializable(typeof(GitRepositoryRefsView))]
[JsonSerializable(typeof(GitRepositoryRefView))]
[JsonSerializable(typeof(GitRepositoryBranchesView))]
[JsonSerializable(typeof(GitRepositoryBranchView))]
[JsonSerializable(typeof(GitRepositoryDirectoryListingView))]
[JsonSerializable(typeof(GitRepositoryEntryView))]
[JsonSerializable(typeof(GitRepositoryFileContentView))]
[JsonSerializable(typeof(GitCommitComparisonView))]
[JsonSerializable(typeof(GitChangedPathView))]
[JsonSerializable(typeof(GitRepositoryComposeDiscovery))]
[JsonSerializable(typeof(GitComposeProjectCandidate))]
[JsonSerializable(typeof(DeleteImagesRequest))]
[JsonSerializable(typeof(DockerHubImageView))]
[JsonSerializable(typeof(ImageView))]
[JsonSerializable(typeof(ImagesView))]
[JsonSerializable(typeof(PullImageInput))]
[JsonSerializable(typeof(IEnumerable<IImageRepository>))]
[JsonSerializable(typeof(IEnumerable<DockerHubTagView>))]
[JsonSerializable(typeof(CreateNetworkInput))]
[JsonSerializable(typeof(CreateNetworkResponse))]
[JsonSerializable(typeof(CreateNetworkView))]
[JsonSerializable(typeof(DeleteNetworksInput))]
[JsonSerializable(typeof(ListNetworksRequest))]
[JsonSerializable(typeof(LookupRequest))]
[JsonSerializable(typeof(GlobalSearchRequest))]
[JsonSerializable(typeof(GlobalSearchResponse))]
[JsonSerializable(typeof(NetworksView))]
[JsonSerializable(typeof(CreateVolumeInput))]
[JsonSerializable(typeof(DeleteVolumesInput))]
[JsonSerializable(typeof(ListVolumesRequest))]
[JsonSerializable(typeof(VolumesView))]
[JsonSerializable(typeof(VolumeDirectoryView))]
[JsonSerializable(typeof(VolumeFileEntryView))]
[JsonSerializable(typeof(IReadOnlyList<VolumeFileEntryView>))]
[JsonSerializable(typeof(VolumeFileEntryType))]
[JsonSerializable(typeof(IEnumerable<DockerHubRepositoryInfo>))]
[JsonSerializable(typeof(DockerHubTag))]
[JsonSerializable(typeof(IEnumerable<DockerHubImage>))]
[JsonSerializable(typeof(GitHubCrPackage))]
[JsonSerializable(typeof(IEnumerable<GitHubCrPackageVersion>))]
[JsonSerializable(typeof(GitHubCrPackageVersionMetadata))]
[JsonSerializable(typeof(GitHubCrPackageVersionContainerMetadata))]
[JsonSerializable(typeof(DockerNetworkResultView))]
[JsonSerializable(typeof(DockerNetworkDetailsView))]
[JsonSerializable(typeof(IpAddressManagementConfig))]
[JsonSerializable(typeof(NetworkConnectedContainer))]
[JsonSerializable(typeof(NetworkPeerInfo))]
[JsonSerializable(typeof(DockerVolumeResultView))]
[JsonSerializable(typeof(ClusterVolume))]
[JsonSerializable(typeof(VolumeUsageData))]
[JsonSerializable(typeof(ClusterVolumeInfo))]
[JsonSerializable(typeof(TopologyEntry))]
[JsonSerializable(typeof(VolumeCapacityRange))]
[JsonSerializable(typeof(DeleteImageResult))]
[JsonSerializable(typeof(IReadOnlyList<DeleteImageResponseItem>))]
[JsonSerializable(typeof(InspectImageResult))]
[JsonSerializable(typeof(ImageRootFs))]
[JsonSerializable(typeof(ImageMetadata))]
[JsonSerializable(typeof(ImageConfig))]
[JsonSerializable(typeof(ImageDescriptor))]
[JsonSerializable(typeof(ImageGraphicDriver))]
[JsonSerializable(typeof(ImageHealthCheck))]
[JsonSerializable(typeof(ImageGraphDriverData))]
[JsonSerializable(typeof(IAsyncEnumerable<PullImageStreamItem>))]
[JsonSerializable(typeof(IEnumerable<DockerHubImageResult>))]
[JsonSerializable(typeof(ContainerInfoView))]
[JsonSerializable(typeof(ExposedPortsResult))]
[JsonSerializable(typeof(CreateContainerView))]
[JsonSerializable(typeof(ContainerAdoptionDraftView))]
[JsonSerializable(typeof(ContainerAdoptionSourceView))]
[JsonSerializable(typeof(ContainerAdoptionIssueView))]
[JsonSerializable(typeof(AdoptContainerInput))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.SwarmServiceAdoptionDraftView))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.AdoptSwarmServiceInput))]
[JsonSerializable(typeof(IAsyncEnumerable<ComposeDeploymentEvent>))]
[JsonSerializable(typeof(RegistryConfigView))]
[JsonSerializable(typeof(IEnumerable<ContainerVolumeResult>))]
[JsonSerializable(typeof(InspectImageView))]
[JsonSerializable(typeof(DeploymentsView))]
[JsonSerializable(typeof(IEnumerable<DeploymentView>))]
[JsonSerializable(typeof(DeploymentView))]
[JsonSerializable(typeof(DeploymentSpec))]
[JsonSerializable(typeof(LocalImage))]
[JsonSerializable(typeof(ExternalImage))]
[JsonSerializable(typeof(BuildImage))]
[JsonSerializable(typeof(SecurityConfig))]
[JsonSerializable(typeof(LoggingConfig))]
[JsonSerializable(typeof(HealthCheckConfig))]
[JsonSerializable(typeof(DeletePlatformsInput))]
[JsonSerializable(typeof(CreateDeploymentInput))]
[JsonSerializable(typeof(DuplicateSourceInput))]
[JsonSerializable(typeof(DeploymentDuplicateDraftView))]
[JsonSerializable(typeof(DuplicateDraftWarningView))]
[JsonSerializable(typeof(DeploymentConfigView))]
[JsonSerializable(typeof(IAsyncEnumerable<DeploymentStreamItem>))]
[JsonSerializable(typeof(IAsyncEnumerable<AutomationActionRunStreamItem>))]
[JsonSerializable(typeof(AutomationActionRunStreamItem))]
[JsonSerializable(typeof(IAsyncEnumerable<StackStreamItem>))]
[JsonSerializable(typeof(ApplyDeploymentInput))]
[JsonSerializable(typeof(StackInput))]
[JsonSerializable(typeof(CreateStackInput))]
[JsonSerializable(typeof(SwarmStackPreflightInput))]
[JsonSerializable(typeof(SwarmStackCompatibilityReport))]
[JsonSerializable(typeof(SwarmStackCompatibilityIssue))]
[JsonSerializable(typeof(StackDuplicateDraftView))]
[JsonSerializable(typeof(ComposeProjectImportDraftView))]
[JsonSerializable(typeof(ComposeProjectImportSourceView))]
[JsonSerializable(typeof(ComposeProjectStackDraftView))]
[JsonSerializable(typeof(ComposeProjectRuntimeService))]
[JsonSerializable(typeof(ComposeProjectImportValidation))]
[JsonSerializable(typeof(ComposeProjectServiceComparison))]
[JsonSerializable(typeof(StackImportKind))]
[JsonSerializable(typeof(StackImportKind?))]
[JsonSerializable(typeof(ValidateComposeProjectImportInput))]
[JsonSerializable(typeof(ImportComposeProjectInput))]
[JsonSerializable(typeof(PatchStackInput))]
[JsonSerializable(typeof(StackInputPatchDocument))]
[JsonSerializable(typeof(StackDriftPolicyInput))]
[JsonSerializable(typeof(StacksView))]
[JsonSerializable(typeof(IEnumerable<StackView>))]
[JsonSerializable(typeof(StackView))]
[JsonSerializable(typeof(StackConfigView))]
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
[JsonSerializable(typeof(StackReleasesView))]
[JsonSerializable(typeof(IEnumerable<StackReleaseView>))]
[JsonSerializable(typeof(StackReleaseView))]
[JsonSerializable(typeof(StackReleaseSource))]
[JsonSerializable(typeof(StackSpec))]
[JsonSerializable(typeof(ManualStack))]
[JsonSerializable(typeof(GitStack))]
[JsonSerializable(typeof(StackBuildImageBinding))]
[JsonSerializable(typeof(IReadOnlyList<StackBuildImageBinding>))]
[JsonSerializable(typeof(WebhookConfig))]
[JsonSerializable(typeof(StackWebhookConfig))]
[JsonSerializable(typeof(AutomationWebhookConfig))]
[JsonSerializable(typeof(StackUpdateState))]
[JsonSerializable(typeof(ManualStackUpdateState))]
[JsonSerializable(typeof(GitStackUpdateState))]
[JsonSerializable(typeof(RecreateStackOnNewImageState))]
[JsonSerializable(typeof(RecreateStackOnNewCommitState))]
[JsonSerializable(typeof(ActivitiesView))]
[JsonSerializable(typeof(ActivityView))]
[JsonSerializable(typeof(ActivityFilter))]
[JsonSerializable(typeof(PagingInput))]
[JsonSerializable(typeof(PagedResultView<ActivityView>))]
[JsonSerializable(typeof(PagedResultView<AlertEventView>))]
[JsonSerializable(typeof(AlertEventView))]
[JsonSerializable(typeof(AlertEventsView))]
[JsonSerializable(typeof(BuildRunFailedAlertInfo))]
[JsonSerializable(typeof(SwarmServiceOperationFailedAlertInfo))]
[JsonSerializable(typeof(LicenseEnteredGracePeriodAlertInfo))]
[JsonSerializable(typeof(LicenseExpiredAlertInfo))]
[JsonSerializable(typeof(AlertEventFilter))]
[JsonSerializable(typeof(ResolveAlertEventsInput))]
[JsonSerializable(typeof(UnresolvedAlertsCountView))]
[JsonSerializable(typeof(PagedResultView<AlertRuleView>))]
[JsonSerializable(typeof(AcknowledgeAlertEventsInput))]
[JsonSerializable(typeof(AlertRuleView))]
[JsonSerializable(typeof(AlertRulesView))]
[JsonSerializable(typeof(AlertChannelView))]
[JsonSerializable(typeof(AlertChannelsView))]
[JsonSerializable(typeof(IEnumerable<AlertChannelView>))]
[JsonSerializable(typeof(AlertRuleInput))]
[JsonSerializable(typeof(CreateAlertRuleInput))]
[JsonSerializable(typeof(PatchAlertRuleInput))]
[JsonSerializable(typeof(AlertRuleInputPatchDocument))]
[JsonSerializable(typeof(AlertChannelInput))]
[JsonSerializable(typeof(VerifyAlertChannelInput))]
[JsonSerializable(typeof(AlertChannelInputPatchDocument))]
[JsonSerializable(typeof(DeleteAlertRulesInput))]
[JsonSerializable(typeof(DeleteAlertChannelsInput))]
[JsonSerializable(typeof(AlertRuleConfigView))]
[JsonSerializable(typeof(WebhookReceiveResult))]
[JsonSerializable(typeof(GitRepositoryConfigView))]
[JsonSerializable(typeof(RoleInput))]
[JsonSerializable(typeof(PermissionInput))]
[JsonSerializable(typeof(UserResourceAccessInput))]
[JsonSerializable(typeof(IEnumerable<UserResourceAccessInput>))]
[JsonSerializable(typeof(ResourceAccessView))]
[JsonSerializable(typeof(IEnumerable<ResourceAccessView>))]
[JsonSerializable(typeof(CreateUserInput))]
[JsonSerializable(typeof(PatchUserInput))]
[JsonSerializable(typeof(AddUserResourceAccessInput))]
[JsonSerializable(typeof(RemoveUserResourceAccessInput))]
[JsonSerializable(typeof(AddTeamResourceAccessInput))]
[JsonSerializable(typeof(RemoveTeamResourceAccessInput))]
[JsonSerializable(typeof(PatchUserInputPatchDocument))]
[JsonSerializable(typeof(AddUserRoleInput))]
[JsonSerializable(typeof(DeleteUsersInput))]
[JsonSerializable(typeof(UsersFilter))]
[JsonSerializable(typeof(UserSearchFilter))]
[JsonSerializable(typeof(UserView))]
[JsonSerializable(typeof(UserSearchItemView))]
[JsonSerializable(typeof(IEnumerable<UserSearchItemView>))]
[JsonSerializable(typeof(UsersView))]
[JsonSerializable(typeof(TeamResourceAccessInput))]
[JsonSerializable(typeof(IEnumerable<TeamResourceAccessInput>))]
[JsonSerializable(typeof(CreateTeamInput))]
[JsonSerializable(typeof(PatchTeamInput))]
[JsonSerializable(typeof(PatchTeamInputPatchDocument))]
[JsonSerializable(typeof(AddTeamRoleInput))]
[JsonSerializable(typeof(AddTeamMemberInput))]
[JsonSerializable(typeof(DeleteTeamsInput))]
[JsonSerializable(typeof(TeamSearchFilter))]
[JsonSerializable(typeof(TeamSearchItemView))]
[JsonSerializable(typeof(IEnumerable<TeamSearchItemView>))]
[JsonSerializable(typeof(TeamsFilter))]
[JsonSerializable(typeof(TeamView))]
[JsonSerializable(typeof(TeamsView))]
[JsonSerializable(typeof(DeleteRolesInput))]
[JsonSerializable(typeof(PatchRolePermissionsInput))]
[JsonSerializable(typeof(PatchRolePermissionsInputPatchDocument))]
[JsonSerializable(typeof(PermissionMatrixViewItem))]
[JsonSerializable(typeof(IReadOnlyDictionary<string, PermissionMatrixViewItem>))]
[JsonSerializable(typeof(PermissionView))]
[JsonSerializable(typeof(RoleView))]
[JsonSerializable(typeof(RolesView))]
[JsonSerializable(typeof(IEnumerable<RoleView>))]
[JsonSerializable(typeof(RenameResource))]
[JsonSerializable(typeof(PatchDeploymentInput))]
[JsonSerializable(typeof(PatchResourceMetadata))]
[JsonSerializable(typeof(PatchGitRepositoryInput))]
[JsonSerializable(typeof(ResourceInfo))]
[JsonSerializable(typeof(ResourceCapabilities))]
[JsonSerializable(typeof(ContainerDataView))]
[JsonSerializable(typeof(ApplyStackInput))]
[JsonSerializable(typeof(RollbackStackInput))]
[JsonSerializable(typeof(ContainersDataView))]
[JsonSerializable(typeof(StackStatsView))]
[JsonSerializable(typeof(StackContainerStatsView))]
[JsonSerializable(typeof(WebhookProvider))]
[JsonSerializable(typeof(WebhookAuthScheme))]
[JsonSerializable(typeof(WebhookExecution))]
[JsonSerializable(typeof(ResourceBindingInput))]
[JsonSerializable(typeof(ResourceBindingView))]
[JsonSerializable(typeof(ResourceBindingsView))]
[JsonSerializable(typeof(UpdateResourceBindingInput))]
[JsonSerializable(typeof(SecretDefinitionView))]
[JsonSerializable(typeof(SecretDefinitionsView))]
[JsonSerializable(typeof(CreateInternalSecretInput))]
[JsonSerializable(typeof(CreateExternalSecretInput))]
[JsonSerializable(typeof(UpdateExternalSecretInput))]
[JsonSerializable(typeof(UpdateExternalSecretPatchDocument))]
[JsonSerializable(typeof(TestExternalSecretInput))]
[JsonSerializable(typeof(ExternalSecretTestResultView))]
[JsonSerializable(typeof(TestVaultKvV2SecretProviderConnectionInput))]
[JsonSerializable(typeof(SecretProviderConnectionTestResultView))]
[JsonSerializable(typeof(SecretProviderView))]
[JsonSerializable(typeof(SecretProvidersView))]
[JsonSerializable(typeof(CreateVaultKvV2SecretProviderInput))]
[JsonSerializable(typeof(UpdateVaultKvV2SecretProviderInput))]
[JsonSerializable(typeof(UpdateVaultKvV2SecretProviderPatchDocument))]
[JsonSerializable(typeof(TagView))]
[JsonSerializable(typeof(TagsView))]
[JsonSerializable(typeof(TagSummaryView))]
[JsonSerializable(typeof(ResourceTagsView))]
[JsonSerializable(typeof(ReplaceResourceTagsInput))]
[JsonSerializable(typeof(CreateTagInput))]
[JsonSerializable(typeof(PatchTagInput))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.CreateSwarmServiceInput))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.UpdateSwarmServiceInput))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.ScaleSwarmServiceInput))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.ManagedSwarmServiceView))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.ManagedSwarmServicesView))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.SwarmServiceDuplicateDraftView))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.Identity.SwarmServiceCapabilities))]
[JsonSerializable(typeof(Domain.Contracts.Resources.SwarmServices.SwarmServiceProgressItem))]
[JsonSerializable(typeof(IAsyncEnumerable<Domain.Contracts.Resources.SwarmServices.SwarmServiceProgressItem>))]
[JsonSerializable(typeof(SwarmServiceSpec))]
[JsonSerializable(typeof(SwarmServiceWebhookConfig))]
[JsonSerializable(typeof(SwarmExternalImage))]
[JsonSerializable(typeof(SwarmBuildImage))]
[JsonSerializable(typeof(WebApi.Routes.Endpoints.Resources.SwarmServices.SwarmServiceOperationView))]

public partial class ApplicationJsonContext : JsonSerializerContext
{
}

[JsonSerializable(typeof(ProblemDetails))]
public partial class ProblemJsonContext : JsonSerializerContext
{

}
