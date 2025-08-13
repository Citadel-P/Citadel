using Citadel.Agent.Common.V1;
using Citadel.Agent.Platforms.V1;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.DockerClient.Models.Platforms;
using Domain.Contracts.Resources.Containers;
using Domain;

namespace Infrastructure.Connectors.Mappers;

internal static class PlatformMappers
{
    public static PlatformResult Map(this PlatformInfoResponse platformInfo, string platformName, string platformAddress)
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

    public static PlatformResult Map(this PlatformInfoResult platformInfo, string platformName, string platformAddress)
    {
        PlatformDescriptor? descriptor = null;
        if (string.IsNullOrEmpty(platformInfo.SwarmInfo?.NodeAddress))
        {
            descriptor = new DockerPlatformDescriptor
            (
                DaemonId: platformInfo.Id,
                Driver: platformInfo.Driver,
                OsType: platformInfo.OsType,
                OsVersion: platformInfo.OsVersion,
                Architecture: platformInfo.Architecture,
                OperatingSystem: platformInfo.OperatingSystem,
                ContainerCount: platformInfo.PlatformStatistics?.ContainerCount ?? 0,
                ContainersPaused: platformInfo.PlatformStatistics?.ContainersPaused ?? 0,
                ContainersRunning: platformInfo.PlatformStatistics?.ContainersRunning ?? 0,
                ContainersStopped: platformInfo.PlatformStatistics?.ContainersStopped ?? 0
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
            MemTotal: platformInfo.MemoryTotal,
            ServerVersion: platformInfo.ServerVersion,
            AgentVersion: platformInfo.AgentVersion,
            Descriptor: descriptor,
            PlatformStat: platformInfo.PlatformStatistics?.Map()
        );
    }

    public static DockerPlatformStat Map(this PlatformStatResult stat)
        => new 
        (
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: stat.MemoryUsage,
            cpuUsage: stat.CpuUsage,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes
        );

    public static void Map(this PlatformStatsResponse source, PlatformStatsResult destination)
    {
        destination.PlatformStat.ReInitialize(
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: source.Stat.MemoryUsage,
            cpuUsage: source.Stat.CpuUsage,
            rxBytes: source.Stat.RxBytes,
            txBytes: source.Stat.TxBytes
        );

        destination.ReInitialize(
            memTotal: source.MemTotal,
            imageCount: source.ImageCount,
            volumeCount: source.VolumeCount,
            networkCount: source.NetworkCount,
            containerCount: source.ContainerCount,
            containersPaused: source.ContainersPaused,
            containersStopped: source.ContainersStopped,
            containersRunning: source.ContainersRunning,
            platformStat: destination.PlatformStat);
    }

    internal static DockerPlatformStat Map(this PlatformStatMessage stat)
        => new
        (
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: stat?.MemoryUsage ?? 0,
            cpuUsage: stat?.CpuUsage ?? 0,
            rxBytes: stat?.RxBytes ?? 0,
            txBytes: stat?.TxBytes ?? 0
        );

    public static void Map (this PlatformStreamResult source, PlatformStatsResult destination)
    {
        destination.PlatformStat.ReInitialize(
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: source.PlatformStatistics.MemoryUsage,
            cpuUsage: source.PlatformStatistics.CpuUsage,
            rxBytes: source.PlatformStatistics.RxBytes,
            txBytes: source.PlatformStatistics.TxBytes
        );

        destination.ReInitialize(
            memTotal: source.MemoryTotal, 
            imageCount: source.ImageCount, 
            volumeCount: source.VolumeCount, 
            networkCount: source.NetworkCount, 
            containerCount: source.PlatformStatistics?.ContainerCount ?? 0, 
            containersPaused: source.PlatformStatistics?.ContainersPaused ?? 0, 
            containersStopped: source.PlatformStatistics?.ContainersStopped ?? 0, 
            containersRunning: source.PlatformStatistics?.ContainersRunning ?? 0,
            platformStat: destination.PlatformStat);
    }

    public static DaemonEventInfo Map(this DaemonEventResult @event)
    {
        return new DaemonEventInfo
        (
            Id: @event.Id,
            Action: @event.Action,
            ContainerId: @event.ContainerId,
            Container: @event.Container?.Map(),
            Type: @event.Type.Map()
        );
    }

    public static ContainerEventType Map(this Hosting.DockerClient.EventMessageType type)
    {
        return type switch
        {
            Hosting.DockerClient.EventMessageType.Container => ContainerEventType.Container,
            Hosting.DockerClient.EventMessageType.Image => ContainerEventType.Image,
            Hosting.DockerClient.EventMessageType.Network => ContainerEventType.Network,
            Hosting.DockerClient.EventMessageType.Volume => ContainerEventType.Volume,
            _ => ContainerEventType.Unknown
        };
    }
}
