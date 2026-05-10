using Application.Features.Images.Queries;
using Domain;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Compose;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Microsoft.AspNetCore.Mvc;
using System.Text.Json.Serialization;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Identity.Actors;
using WebApi.Routes.Endpoints.Resources.Identity.Users;
using WebApi.Routes.Endpoints.Resources.Identity.Teams;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitAccounts;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Networks;
using WebApi.Routes.Endpoints.Resources.Paging;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.Stacks;
using WebApi.Routes.Endpoints.Resources.Volumes;
using WebApi.Routes.Endpoints.Resources.Lookup;
using DeleteImageResponseItem = Domain.Contracts.Resources.Images.DeleteImageResponseItem;
using ApiActorView = WebApi.Routes.Endpoints.Resources.Identity.Actors.ActorView;
using WebApi.Routes.Endpoints.Resources.Identity.Roles;
using WebApi.Routes.Endpoints.Resources.Identity.Auth;
using Application.Features.Identity.Auth.Models;
using Application.Features.Identity.Auth.Models;
using Domain.Contracts.Resources;
using Hosting.Common;

namespace Application.Models;

[JsonSourceGenerationOptions(
    GenerationMode = JsonSourceGenerationMode.Default,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<ActorType>)
    })]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(Guid[]))]
[JsonSerializable(typeof(ResourceType))]
[JsonSerializable(typeof(ResourceType?))]
[JsonSerializable(typeof(ActorType))]
[JsonSerializable(typeof(ActorType?))]
[JsonSerializable(typeof(ApiActorView))]
[JsonSerializable(typeof(PatchActorEnabledInput))]
[JsonSerializable(typeof(List<Platform>))]
[JsonSerializable(typeof(List<PlatformStat>))]
[JsonSerializable(typeof(SwarmPeer))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(List<ContainerView>))]
[JsonSerializable(typeof(IDictionary<string, IReadOnlyList<HostPortBinding>>))]
[JsonSerializable(typeof(EndpointSettingsView))]
[JsonSerializable(typeof(LoginRequest))]
[JsonSerializable(typeof(LoginResponse))]
[JsonSerializable(typeof(RefreshTokenResponse))]
[JsonSerializable(typeof(PlatformInput))]
[JsonSerializable(typeof(PlatformsView))]
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
[JsonSerializable(typeof(GitRepository))]
[JsonSerializable(typeof(CreateGitRepositoryInput))]
[JsonSerializable(typeof(GitRepositoryInputPatchDocument))]
[JsonSerializable(typeof(DeleteGitRepositoriesInput))]
[JsonSerializable(typeof(GitRepositoriesView))]
[JsonSerializable(typeof(GitRepositoryView))]
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
[JsonSerializable(typeof(NetworksView))]
[JsonSerializable(typeof(CreateVolumeInput))]
[JsonSerializable(typeof(DeleteVolumesInput))]
[JsonSerializable(typeof(ListVolumesRequest))]
[JsonSerializable(typeof(VolumesView))]
[JsonSerializable(typeof(EndpointMetadata))]
[JsonSerializable(typeof(IEnumerable<DockerHubRepositoryInfo>))]
[JsonSerializable(typeof(DockerHubTag))]
[JsonSerializable(typeof(IEnumerable<DockerHubImage>))]
[JsonSerializable(typeof(GitHubCrPackage))]
[JsonSerializable(typeof(IEnumerable<GitHubCrPackageVersion>))]
[JsonSerializable(typeof(GitHubCrPackageVersionMetadata))]
[JsonSerializable(typeof(GitHubCrPackageVersionContainerMetadata))]
[JsonSerializable(typeof(DockerNetworkResult))]
[JsonSerializable(typeof(DockerNetworkDetails))]
[JsonSerializable(typeof(IpAddressManagementConfig))]
[JsonSerializable(typeof(NetworkConnectedContainer))]
[JsonSerializable(typeof(NetworkPeerInfo))]
[JsonSerializable(typeof(DockerVolumeResult))]
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
[JsonSerializable(typeof(IAsyncEnumerable<ComposeDeploymentEvent>))]
[JsonSerializable(typeof(RegistryConfigView))]
[JsonSerializable(typeof(IEnumerable<ContainerVolumeResult>))]
[JsonSerializable(typeof(InspectImageView))]
[JsonSerializable(typeof(DeploymentsView))]
[JsonSerializable(typeof(IEnumerable<DeploymentView>))]
[JsonSerializable(typeof(DeploymentView))]
[JsonSerializable(typeof(DeploymentSpec))]
[JsonSerializable(typeof(SecurityConfig))]
[JsonSerializable(typeof(LoggingConfig))]
[JsonSerializable(typeof(HealthCheckConfig))]
[JsonSerializable(typeof(DeletePlatformsInput))]
[JsonSerializable(typeof(CreateDeploymentInput))]
[JsonSerializable(typeof(DeploymentConfigView))]
[JsonSerializable(typeof(IAsyncEnumerable<DeploymentStreamItem>))]
[JsonSerializable(typeof(ApplyDeploymentInput))]
[JsonSerializable(typeof(StackInput))]
[JsonSerializable(typeof(CreateStackInput))]
[JsonSerializable(typeof(PatchStackInput))]
[JsonSerializable(typeof(StackInputPatchDocument))]
[JsonSerializable(typeof(StacksView))]
[JsonSerializable(typeof(IEnumerable<StackView>))]
[JsonSerializable(typeof(StackView))]
[JsonSerializable(typeof(StackConfigView))]
[JsonSerializable(typeof(StackReleasesView))]
[JsonSerializable(typeof(IEnumerable<StackReleaseView>))]
[JsonSerializable(typeof(StackReleaseView))]
[JsonSerializable(typeof(StackSpec))]
[JsonSerializable(typeof(ManualStack))]
[JsonSerializable(typeof(GitStack))]
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
public partial class ApplicationJsonContext : JsonSerializerContext
{
}

[JsonSerializable(typeof(ProblemDetails))]
public partial class ProblemJsonContext : JsonSerializerContext
{

}