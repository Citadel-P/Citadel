using Citadel.Agent.Common.V1;
using Citadel.Agent.Platforms.V1;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;

namespace Infrastructure.Connectors.Mappers;

internal static class PlatformMapper
{
    public static PlatformResult Map(this PlatformInfoResponse platformInfo, string platformName, string platformAddress, PlatformConnectorType type)
    {
        PlatformDescriptor? descriptor = null;
        if (string.IsNullOrEmpty(platformInfo.SwarmInfo?.NodeID))
        {
            descriptor = new DockerPlatformDescriptor
            (
                DaemonId: platformInfo.Id,
                Driver: platformInfo.Driver,
                OsType: platformInfo.OsType,
                OsVersion: platformInfo.OsVersion,
                Architecture: platformInfo.Architecture,
                ContainerCount: platformInfo.ContainerCount,
                OperatingSystem: platformInfo.OperatingSystem,
                ContainersPaused: platformInfo.ContainersPaused,
                ContainersRunning: platformInfo.ContainersRunning,
                ContainersStopped: platformInfo.ContainersStopped
            );
        }
        else
        {
            // Todo : Implement Swarm descriptor 
        }
        return new PlatformResult
            (
                Name: platformName,
                Address: platformAddress,
                NetworkCount: platformInfo.NetworkCount,
                VolumeCount: platformInfo.VolumeCount,
                ImageCount: platformInfo.ImageCount,
                CpuCount: platformInfo.CpuCount,
                MemTotal: platformInfo.MemTotal,
                ServerVersion: platformInfo.ServerVersion,
                AgentVersion: platformInfo.AgentVersion,
                Descriptor: descriptor,
                PlatformStat: platformInfo.PlatformStat.Map()
            );
    }

    public static PlatformStatsResult Map(this PlatformStatsResponse stat)
        => new 
        (
            MemTotal: stat.MemTotal,
            ImageCount: stat.ImageCount,
            VolumeCount: stat.VolumeCount,
            NetworkCount: stat.NetworkCount,
            ContainerCount: stat.ContainerCount,
            ContainersPaused: stat.ContainersPaused,
            ContainersStopped: stat.ContainersStopped,
            ContainersRunning: stat.ContainersRunning,
            PlatformStat: stat.Stat.Map()
        );

    internal static DockerPlatformStat Map(this PlatformStatMessage stat)
        => new
        (
            Created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            MemoryUsage: stat?.MemoryUsage ?? 0,
            CpuUsage: stat?.CpuUsage ?? 0,
            RxBytes: stat?.RxBytes ?? 0,
            TxBytes: stat?.TxBytes ?? 0
        );
}
