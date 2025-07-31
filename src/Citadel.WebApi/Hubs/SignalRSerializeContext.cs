using System.Text.Json.Serialization;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Platforms;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(ContainersView))]
[JsonSerializable(typeof(ContainerView))]
[JsonSerializable(typeof(List<ContainerView>))]
[JsonSerializable(typeof(List<ContainerStatView>))]
[JsonSerializable(typeof(IEnumerable<ContainerStatView>))]
[JsonSerializable(typeof(ContainerStatView))]
[JsonSerializable(typeof(PortView))]
[JsonSerializable(typeof(List<PortView>))]
[JsonSerializable(typeof(Dictionary<string, string>))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(PlatformView))]
[JsonSerializable(typeof(PlatformDescriptor))]
[JsonSerializable(typeof(DockerPlatformDescriptor))]
[JsonSerializable(typeof(DockerSwarmPlatformDescriptor))]
[JsonSerializable(typeof(KubernetesPlatformDescriptor))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(ContainerLogView))]
[JsonSerializable(typeof(PlatformsView))]
[JsonSerializable(typeof(PlatformStatsBatchView))]
[JsonSerializable(typeof(PlatformStatView))]
[JsonSerializable(typeof(DockerContainer))]
[JsonSerializable(typeof(DockerContainerStat))]
internal partial class SignalRSerializeContext : JsonSerializerContext
{
}
