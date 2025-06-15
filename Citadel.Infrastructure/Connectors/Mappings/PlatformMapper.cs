using Citadel.Agent.Common.V1;
using Citadel.Agent.Platforms.V1;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;

namespace Infrastructure.Connectors.Mappings;

internal static class PlatformMapper
{
    public static Platform Map(this PlatformInfoResponse platformInfo, string platformName, string platformAddress, PlatformConnectorType type)
    {
        PlatformDescriptor? descriptor = null;
        if (string.IsNullOrEmpty(platformInfo.SwarmInfo.NodeID))
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
        }
        return new Platform
            (
                name: platformName,
                connectorType: type,
                address: platformAddress,
                status: PlatformStatus.Online,
                networkCount: platformInfo.NetworkCount,
                volumeCount: platformInfo.VolumeCount,
                imageCount: platformInfo.ImageCount,
                cpuCount: platformInfo.CpuCount,
                memTotal: platformInfo.MemTotal,
                serverVersion: platformInfo.ServerVersion,
                agentVersion: platformInfo.AgentVersion,
                platformDescriptor: descriptor
            ).AppendStat(platformInfo.PlatformStat.Map(created: platformInfo.Created));
    }

    public static PlatformStatsBatch Map(this PlatformStatsResponse stat, Guid PlatformId)
        => new 
        (
            PlatformId: PlatformId,
            MemTotal: stat.MemTotal,
            ImageCount: stat.ImageCount,
            VolumeCount: stat.VolumeCount,
            NetworkCount: stat.NetworkCount,
            ContainerCount: stat.ContainerCount,
            ContainersPaused: stat.ContainersPaused,
            ContainersStopped: stat.ContainersStopped,
            ContainersRunning: stat.ContainersRunning,
            PlatformStat: stat.Stat.Map(PlatformId)
        );

    internal static PlatformStat Map(this PlatformStatMessage stat, Guid? platformId = null, long? created = null)
        => new
        (
            created: created ?? DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            platformId: platformId ?? Guid.Empty,
            memoryUsage: stat?.MemoryUsage ?? 0,
            cpuUsage: stat?.CpuUsage ?? 0,
            rxBytes: stat?.RxBytes ?? 0,
            txBytes: stat?.TxBytes ?? 0
        );
}
