using System.Text.Json.Serialization;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(ContainersInfoView))]
[JsonSerializable(typeof(ContainerInfoView))]
[JsonSerializable(typeof(List<ContainerInfoView>))]
[JsonSerializable(typeof(List<ContainerStatView>))]
[JsonSerializable(typeof(IEnumerable<ContainerStatView>))]
[JsonSerializable(typeof(ContainerStatView))]
[JsonSerializable(typeof(PortView))]
[JsonSerializable(typeof(List<PortView>))]
[JsonSerializable(typeof(Dictionary<string, string>))]
[JsonSerializable(typeof(IEnumerable<PlatformView>))]
[JsonSerializable(typeof(PlatformView))]
[JsonSerializable(typeof(List<PlatformStatView>))]
[JsonSerializable(typeof(SwarmInfoView))]
[JsonSerializable(typeof(List<SwarmPeerView>))]
[JsonSerializable(typeof(ContainerLogView))]
[JsonSerializable(typeof(PlatformsView))]
[JsonSerializable(typeof(PlatformStatsBatchView))]
[JsonSerializable(typeof(PlatformStatView))]
internal partial class SignalRSerializeContext : JsonSerializerContext
{
}
