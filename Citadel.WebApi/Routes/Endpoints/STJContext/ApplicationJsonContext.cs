using System.Text.Json.Serialization;
using Application.Features.Auth.Models;
using Application.Features.Containers.Models;
using Application.Features.Platforms.Models;
using Infrastructure.Entities;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Auth;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace Application.Models;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(List<Platform>))]
[JsonSerializable(typeof(List<PlatformStat>))]
[JsonSerializable(typeof(SwarmPeer))]
[JsonSerializable(typeof(SystemInfoView))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(Infrastructure.AppPermission[]))]
[JsonSerializable(typeof(DockerHubRegistry))]
[JsonSerializable(typeof(AzureRegistry))]
[JsonSerializable(typeof(AWSRegistry))]
[JsonSerializable(typeof(GitlabRegistry))]
[JsonSerializable(typeof(CustomRegistry))]
[JsonSerializable(typeof(List<ContainerInfoView>))]
[JsonSerializable(typeof(List<PortView>))]
[JsonSerializable(typeof(NetworkSettingsView))]
[JsonSerializable(typeof(EndpointSettingsView))]
[JsonSerializable(typeof(StreamLogsRequest))]
[JsonSerializable(typeof(LoginRequest))]
[JsonSerializable(typeof(LoginResponse))]
[JsonSerializable(typeof(PlatformsView))]
[JsonSerializable(typeof(ContainersInfoView))]
[JsonSerializable(typeof(ContainerInfoView))]
[JsonSerializable(typeof(SystemInfoRequest))]
[JsonSerializable(typeof(List<SwarmPeerRequest>))]
[JsonSerializable(typeof(SwarmInfoRequest))]
[JsonSerializable(typeof(ContainersInfoRequest))]
[JsonSerializable(typeof(List<PortRequest>))]
[JsonSerializable(typeof(ContainerStatRequest))]
[JsonSerializable(typeof(ContainerEventRequest))]
[JsonSerializable(typeof(ContainerLogRequest))]
[JsonSerializable(typeof(PutPlatformRequest))]
[JsonSerializable(typeof(ContainerStatsView))]
[JsonSerializable(typeof(ContainerStatView))]
public partial class ApplicationJsonContext : JsonSerializerContext
{
}

[JsonSerializable(typeof(ProblemDetails))]
public partial class ProblemJsonContext : JsonSerializerContext
{

}