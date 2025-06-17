using System.Text.Json.Serialization;
using Application.Features.Auth.Models;
using Application.Features.Images.Queries;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Auth;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Networks;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.Volumes;
using DeleteImageResponseItem = Domain.Contracts.Resources.Images.DeleteImageResponseItem;

namespace Application.Models;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(List<Platform>))]
[JsonSerializable(typeof(List<PlatformStat>))]
[JsonSerializable(typeof(SwarmPeer))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(List<ContainerView>))]
[JsonSerializable(typeof(List<PortView>))]
[JsonSerializable(typeof(EndpointSettingsView))]
[JsonSerializable(typeof(StreamLogsRequest))]
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
[JsonSerializable(typeof(RegistryInputPatchDocument))]
[JsonSerializable(typeof(DeleteRegistriesInput))]
[JsonSerializable(typeof(RegistriesView))]
[JsonSerializable(typeof(RegistryView))]
[JsonSerializable(typeof(RegistryConfigurationBase))]
[JsonSerializable(typeof(DeleteImagesRequest))]
[JsonSerializable(typeof(DockerHubImageView))]
[JsonSerializable(typeof(ImageView))]
[JsonSerializable(typeof(ImagesView))]
[JsonSerializable(typeof(PullImageRequest))]
[JsonSerializable(typeof(IEnumerable<IImageRepository>))]
[JsonSerializable(typeof(IEnumerable<DockerHubTagView>))]
[JsonSerializable(typeof(CreateNetworkInput))]
[JsonSerializable(typeof(CreateNetworkResponse))]
[JsonSerializable(typeof(CreateNetworkView))]
[JsonSerializable(typeof(DeleteNetworksInput))]
[JsonSerializable(typeof(ListNetworksRequest))]
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
[JsonSerializable(typeof(ContainerLogInfo))]
[JsonSerializable(typeof(DockerNetwork))]
[JsonSerializable(typeof(DockerNetworkDetails))]
[JsonSerializable(typeof(IpAddressManagementConfig))]
[JsonSerializable(typeof(NetworkConnectedContainer))]
[JsonSerializable(typeof(NetworkPeerInfo))]
[JsonSerializable(typeof(DockerVolume))]
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
[JsonSerializable(typeof(IAsyncEnumerable<PullImageResult>))]
[JsonSerializable(typeof(IEnumerable<DockerHubImageResult>))]
public partial class ApplicationJsonContext : JsonSerializerContext
{
}

[JsonSerializable(typeof(ProblemDetails))]
public partial class ProblemJsonContext : JsonSerializerContext
{

}