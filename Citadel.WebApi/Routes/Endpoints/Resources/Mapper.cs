using Agent.Server.Containers;
using Application.Features.Platforms.Commands;
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
}
