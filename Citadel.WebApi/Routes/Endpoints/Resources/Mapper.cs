using Agent.Server.Containers;
using Citadel.Common;
using Google.Protobuf.Collections;
using Infrastructure.Entities;
using Riok.Mapperly.Abstractions;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources;

[Mapper(RequiredMappingStrategy = RequiredMappingStrategy.Target)]
internal static partial class Mapper
{
    public static partial IEnumerable<PlatformView> Map(IEnumerable<Platform> platforms);
    [MapPropertyFromSource(nameof(PlatformView.Status), Use = nameof(MapCreatedToPlatformStatus))]
    public static partial PlatformView Map(Platform platform);
    public static partial ContainerInfoView Map(ContainerInfo containerInfo);
    public static partial PortView Map(ContainerPort port);

    [MapProperty(nameof(ContainerStat.Created), nameof(ContainerStatView.Created), Use = nameof(MapCreatedToShortDate))]
    public static partial ContainerStatView Map(ContainerStat stat);
    public static partial ContainerLogView Map(ContainerLogReply reply);

    private static string MapCreatedToShortDate(long timeStamp)
        => DateTimeOffset.FromUnixTimeSeconds(timeStamp).UtcDateTime.ToString("HH:mm:ss");

    private static PlatformStatus MapCreatedToPlatformStatus(Platform platform)
    {
        if (platform.Stats.Count == 0) return PlatformStatus.Disconnected;

        var created = DateTimeOffset.FromUnixTimeSeconds(platform.Stats.Last().Created);
        if (created.AddSeconds(30) > DateTimeOffset.UtcNow)
            return PlatformStatus.Connected;
        else return PlatformStatus.Disconnected;
    }

    public static partial ContainerInspectView Map(this ContainerInspectReply containerInfo);

    [MapProperty(nameof(NetworkSettings.Networks), nameof(NetworkSettingsView.Networks), Use = nameof(ToDictionary))]
    public static partial NetworkSettingsView Map(this NetworkSettings networkSettings);
    public static partial EndpointSettingsView Map(this EndpointSettings endpointSettings);

    private static Dictionary<string, EndpointSettingsView> ToDictionary(RepeatedField<MapFieldNetwork> networks)
    {
        var result = new Dictionary<string, EndpointSettingsView>();
        foreach (var network in networks)
        {
            var endpointSettings = network.Value;
            result[network.Key] = endpointSettings.Map();
        }
        return result;
    }

    internal static IEnumerable<PortBindingView> Map(this IEnumerable<PortBinding> portsBinding) => portsBinding.Select(Map);
    internal static PortBindingView Map(this PortBinding portBinding)
        => new(portBinding.HostIP, portBinding.HostPort);

    internal static MapFieldPortBindingView Map(MapFieldPortBinding mapFieldPortBinding)
        => new(mapFieldPortBinding.Key, mapFieldPortBinding.Value.Map());
}
