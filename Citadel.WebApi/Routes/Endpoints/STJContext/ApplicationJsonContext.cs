using System.Text.Json.Serialization;
using Citadel.Agent.Containers.V1;
using Citadel.Agent.Images.V1;
using Application.Features.Auth.Models;
using Application.Features.Images.Queries;
using Citadel.Agent.Common.V1;
using Infrastructure.DockerHub;
using Infrastructure.Entities;
using Infrastructure.Entities.Platforms;
using Infrastructure.Entities.Registries;
using Infrastructure.GithubCr;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Auth;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Networks;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.Volumes;

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
[JsonSerializable(typeof(ContainerState))]
[JsonSerializable(typeof(HostConfig))]
[JsonSerializable(typeof(GraphDriverData))]
[JsonSerializable(typeof(MountPoint))]
[JsonSerializable(typeof(ContainerConfig))]
[JsonSerializable(typeof(NetworkSettingsView))]
[JsonSerializable(typeof(MapFieldPortBindingView))]
[JsonSerializable(typeof(Address))]
[JsonSerializable(typeof(DeleteContainersRequest))]
[JsonSerializable(typeof(RegistryInput))]
[JsonSerializable(typeof(RegistryInputPatchDocument))]
[JsonSerializable(typeof(DeleteRegistriesInput))]
[JsonSerializable(typeof(RegistriesView))]
[JsonSerializable(typeof(RegistryView))]
[JsonSerializable(typeof(RegistryConfigurationBase))]
[JsonSerializable(typeof(IAsyncEnumerable<PullImageReply>))]
[JsonSerializable(typeof(DeleteImagesRequest))]
[JsonSerializable(typeof(DockerHubImageView))]
[JsonSerializable(typeof(ImageView))]
[JsonSerializable(typeof(ImagesView))]
[JsonSerializable(typeof(InspectImageView))]
[JsonSerializable(typeof(PullImageRequest))]
[JsonSerializable(typeof(IEnumerable<IImageRepository>))]
[JsonSerializable(typeof(IEnumerable<GhcrPackageVersion>))]
[JsonSerializable(typeof(IEnumerable<DockerHubRepository>))]
[JsonSerializable(typeof(IEnumerable<DockerHubTagView>))]
[JsonSerializable(typeof(IEnumerable<DockerHubImageModel>))]
[JsonSerializable(typeof(DeleteImagesReply))]
[JsonSerializable(typeof(CreateNetworkInput))]
[JsonSerializable(typeof(CreateNetworkResponse))]
[JsonSerializable(typeof(CreateNetworkView))]
[JsonSerializable(typeof(DeleteNetworksInput))]
[JsonSerializable(typeof(InspectNetworkView))]
[JsonSerializable(typeof(ListNetworksRequest))]
[JsonSerializable(typeof(NetworksView))]
[JsonSerializable(typeof(NetworkView))]
[JsonSerializable(typeof(CreateVolumeInput))]
[JsonSerializable(typeof(DeleteVolumesInput))]
[JsonSerializable(typeof(InspectVolumeView))]
[JsonSerializable(typeof(ListVolumesRequest))]
[JsonSerializable(typeof(VolumesView))]
[JsonSerializable(typeof(VolumeView))]
[JsonSerializable(typeof(IAsyncEnumerable<ContainerLogReply>))]
[JsonSerializable(typeof(EndpointMetadata))]
public partial class ApplicationJsonContext : JsonSerializerContext
{
}

[JsonSerializable(typeof(ProblemDetails))]
public partial class ProblemJsonContext : JsonSerializerContext
{

}