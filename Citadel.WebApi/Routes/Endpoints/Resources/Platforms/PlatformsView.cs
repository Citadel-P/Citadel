using Infrastructure;
using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformView(
    Guid Id,
    string Name,
    string Address,
    PlatformStatus Status,
    string DaemonId,
    int NetworksCount,
    int VolumesCount,
    long Containers,
    long ContainersRunning,
    long ContainersPaused,
    long ContainersStopped,
    long Images,
    string Driver,
    string OperatingSystem,
    string OsVersion,
    string OsType,
    string Architecture,
    long Ncpu,
    long MemTotal,
    string ServerVersion,
    string AgentVersion,
    SwarmInfoView SwarmInfo,
    IEnumerable<PlatformStatView> Stats
    )
{
    internal static IEnumerable<PlatformView> Map(IEnumerable<Platform> platforms)
        => platforms.Select(Map);

    internal static PlatformView Map(Platform platform)
        => Mapper.Map(platform);
}

public sealed record PlatformsView(IEnumerable<PlatformView> Platforms)
{
    internal static PlatformsView Map(IEnumerable<Platform> platforms)
       => new(PlatformView.Map(platforms));
}