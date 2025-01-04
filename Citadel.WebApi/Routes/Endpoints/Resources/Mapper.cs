using Application.Features.Containers.Models;
using Infrastructure.Entities;
using Riok.Mapperly.Abstractions;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources;

[Mapper(RequiredMappingStrategy = RequiredMappingStrategy.Target)]
internal static partial class Mapper
{
    public static partial PlatformView Map(Platform platform);
    public static partial ContainerInfoView Map(ContainerInfo containerInfo);
    public static partial PortView Map(ContainerPort port);

    [MapProperty(nameof(ContainerStat.Created), nameof(ContainerStatView.Created), Use = nameof(MapCreatedToShortDate))]
    public static partial ContainerStatView Map(ContainerStat stat);
    public static partial ContainerLogView Map(ContainerLogRequest logs);

    private static string MapCreatedToShortDate(long timeStamp)
        => DateTimeOffset.FromUnixTimeSeconds(timeStamp).UtcDateTime.ToString("HH:mm:ss");
}
