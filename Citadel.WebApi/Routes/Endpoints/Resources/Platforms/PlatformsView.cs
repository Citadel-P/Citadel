using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record PlatformView(
    Guid Id,
    string Name,
    string Address,
    PlatformStatus Status,
    SystemInfoView SystemInfo,
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

public enum PlatformStatus
{
    Disconnected,
    Connected
}