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
            Created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            MemoryUsage: stat.MemoryUsage,
            CpuUsage: stat.CpuUsage,
            RxBytes: stat.RxBytes,
            TxBytes: stat.TxBytes
        );

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

    public static PlatformStatsResult Map (this PlatformStreamResult stat)
        => new 
        (
            MemTotal: stat.MemoryTotal,
            ImageCount: stat.ImageCount,
            VolumeCount: stat.VolumeCount,
            NetworkCount: stat.NetworkCount,
            ContainerCount: stat.ContainerCount,
            ContainersPaused: stat.ContainersPaused,
            ContainersStopped: stat.ContainersStopped,
            ContainersRunning: stat.ContainersRunning,
            PlatformStat: stat.PlatformStatistics.Map()
        );

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
