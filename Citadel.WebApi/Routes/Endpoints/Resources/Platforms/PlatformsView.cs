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
    string? Driver,
    string? OperatingSystem,
    string? OsVersion,
    string? OsType,
    string? Architecture,
    long Ncpu,
    long MemTotal,
    string? ServerVersion,
    string? AgentVersion,
    SwarmInfoView? SwarmInfo,
    IEnumerable<PlatformStatView>? Stats
    )
{
    internal static IEnumerable<PlatformView> Map(IEnumerable<Platform> platforms)
        => platforms.Select(Map);

    internal static PlatformView Map(Platform platform)
        => platform.Map();
}

public sealed record PlatformsView(IEnumerable<PlatformView> Platforms)
{
    internal static PlatformsView Map(IEnumerable<Platform> platforms)
       => new(PlatformView.Map(platforms));
}

internal static class PlatformMapperExtension
{
    internal static PlatformView Map(this Platform platform) => new(
        Id: platform.Id,
        Name: platform.Name,
        Address: platform.Address,
        Status: platform.Status,
        DaemonId: platform.DaemonId,
        NetworksCount: platform.NetworksCount,
        VolumesCount: platform.VolumesCount,
        Containers: platform.Containers,
        ContainersRunning: platform.ContainersRunning,
        ContainersPaused: platform.ContainersPaused,
        ContainersStopped: platform.ContainersStopped,
        Images: platform.Images,
        Driver: platform.Driver,
        OperatingSystem: platform.OperatingSystem,
        OsVersion: platform.OsVersion,
        OsType: platform.OsType,
        Architecture: platform.Architecture,
        Ncpu: platform.Ncpu,
        MemTotal: platform.MemTotal,
        ServerVersion: platform.ServerVersion,
        AgentVersion: platform.AgentVersion,
        SwarmInfo: platform.SwarmInfo?.Map(),
        Stats: platform.Stats?.Select(Map));

    internal static SwarmInfoView Map(this SwarmInfo swarmInfo) => new (
        Id: swarmInfo.Id,
        NodeID: swarmInfo.NodeID,
        NodeAddr: swarmInfo.NodeAddr,
        LocalNodeState: swarmInfo.LocalNodeState,
        ControlAvailable: swarmInfo.ControlAvailable,
        Error: swarmInfo.Error,
        Nodes: swarmInfo.Nodes,
        Managers: swarmInfo.Managers,
        RemoteManagers: swarmInfo.RemoteManagers is null ? null : [.. swarmInfo.RemoteManagers.Select(x => new SwarmPeerView(x.NodeID, x.Addr))]
        );

    internal static PlatformStatView Map(this PlatformStat stat) => new (
            MemoryUsage: stat.MemoryUsage,
            CpuUsage: stat.CpuUsage,
            Created: stat.Created,
            RxBytes: stat.RxBytes,
            TxBytes: stat.TxBytes);
}