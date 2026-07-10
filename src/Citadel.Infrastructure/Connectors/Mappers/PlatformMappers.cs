using Citadel.SharedModels.V1;
using Citadel.Platforms.V1;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.DockerClient.Models.Platforms;
using Domain.Contracts.Resources.Containers;
using Domain;
using DomainPruneResource = Domain.PruneResource;
using DockerPruneResource = Hosting.DockerClient.Models.Platforms.DockerPruneResource;
using DockerPruneResult = Hosting.DockerClient.Models.Platforms.PlatformPruneResult;
using ProtoPruneResource = Citadel.Platforms.V1.PruneResource;

namespace Infrastructure.Connectors.Mappers;

internal static class PlatformMappers
{
    internal static PlatformResult Map(this PlatformInfoResponse platformInfo, string platformName, string platformAddress)
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
                OperatingSystem: platformInfo.OperatingSystem,
                ContainerCount: platformInfo?.PlatformStat?.ContainerCount ?? 0,
                ContainersPaused: platformInfo?.PlatformStat?.ContainersPaused ?? 0,
                ContainersRunning: platformInfo?.PlatformStat?.ContainersRunning ?? 0,
                ContainersStopped: platformInfo?.PlatformStat?.ContainersStopped ?? 0
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

    internal static PlatformResult Map(this PlatformInfoResult platformInfo, string platformName, string platformAddress)
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

    internal static DockerPlatformStat Map(this PlatformStatResult stat)
        => new 
        (
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: stat.MemoryUsage,
            cpuUsage: stat.CpuUsage,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes,
            containerCount: stat.ContainerCount,
            containersPaused: stat.ContainersPaused,
            containersStopped: stat.ContainersStopped,
            containersRunning: stat.ContainersRunning
        );

    internal static PlatformStatsResult Map(this PlatformStatsResponse source)
    {
        var platformStat = new DockerPlatformStat(
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: source.Stat.MemoryUsage,
            cpuUsage: source.Stat.CpuUsage,
            rxBytes: source.Stat.RxBytes,
            txBytes: source.Stat.TxBytes,
            containerCount: source.Stat.ContainerCount,
            containersPaused: source.Stat.ContainersPaused,
            containersStopped: source.Stat.ContainersStopped,
            containersRunning: source.Stat.ContainersRunning
        );

        return new (
            MemTotal: source.MemTotal,
            ImageCount: source.ImageCount,
            VolumeCount: source.VolumeCount,
            NetworkCount: source.NetworkCount,
            AgentVersion: source.AgentVersion,
            PlatformStat: platformStat);
    }

    internal static DockerPlatformStat Map(this PlatformStatMessage stat)
        => new
        (
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: stat?.MemoryUsage ?? 0,
            cpuUsage: stat?.CpuUsage ?? 0,
            rxBytes: stat?.RxBytes ?? 0,
            txBytes: stat?.TxBytes ?? 0,
            containerCount: stat?.ContainerCount ?? 0,
            containersPaused: stat?.ContainersPaused ?? 0,
            containersStopped: stat?.ContainersStopped ?? 0,
            containersRunning: stat?.ContainersRunning ?? 0
        );

    internal static PlatformStatsResult Map (this PlatformStreamResult source)
    {
        DockerPlatformStat platformStat = new (
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            memoryUsage: source.PlatformStatistics?.MemoryUsage ?? 0,
            cpuUsage: source.PlatformStatistics?.CpuUsage ?? 0,
            rxBytes: source.PlatformStatistics?.RxBytes ?? 0,
            txBytes: source.PlatformStatistics?.TxBytes ?? 0,
            containerCount: source.PlatformStatistics?.ContainerCount ?? 0,
            containersPaused: source.PlatformStatistics?.ContainersPaused ?? 0,
            containersStopped: source.PlatformStatistics?.ContainersStopped ?? 0,
            containersRunning: source.PlatformStatistics?.ContainersRunning ?? 0
        );

        return new PlatformStatsResult(
            MemTotal: source.MemoryTotal, 
            ImageCount: source.ImageCount, 
            VolumeCount: source.VolumeCount, 
            NetworkCount: source.NetworkCount,
            AgentVersion: string.Empty,
            PlatformStat: platformStat);
    }

    internal static DockerPruneResource Map(this DomainPruneResource resource)
        => resource switch
        {
            DomainPruneResource.All => DockerPruneResource.All,
            DomainPruneResource.Volume => DockerPruneResource.Volume,
            DomainPruneResource.Network => DockerPruneResource.Network,
            DomainPruneResource.Image => DockerPruneResource.Image,
            DomainPruneResource.Build => DockerPruneResource.Build,
            _ => DockerPruneResource.All
        };

    internal static ProtoPruneResource MapToProto(this DomainPruneResource resource)
        => resource switch
        {
            DomainPruneResource.All => ProtoPruneResource.All,
            DomainPruneResource.Volume => ProtoPruneResource.Volume,
            DomainPruneResource.Network => ProtoPruneResource.Network,
            DomainPruneResource.Image => ProtoPruneResource.Image,
            DomainPruneResource.Build => ProtoPruneResource.Build,
            _ => ProtoPruneResource.Unspecified
        };

    internal static DomainPruneResource Map(this DockerPruneResource resource)
        => resource switch
        {
            DockerPruneResource.All => DomainPruneResource.All,
            DockerPruneResource.Volume => DomainPruneResource.Volume,
            DockerPruneResource.Network => DomainPruneResource.Network,
            DockerPruneResource.Image => DomainPruneResource.Image,
            DockerPruneResource.Build => DomainPruneResource.Build,
            _ => DomainPruneResource.All
        };

    internal static DomainPruneResource Map(this ProtoPruneResource resource)
        => resource switch
        {
            ProtoPruneResource.All => DomainPruneResource.All,
            ProtoPruneResource.Volume => DomainPruneResource.Volume,
            ProtoPruneResource.Network => DomainPruneResource.Network,
            ProtoPruneResource.Image => DomainPruneResource.Image,
            ProtoPruneResource.Build => DomainPruneResource.Build,
            _ => DomainPruneResource.All
        };

    internal static ProtoPruneResource MapToProto(this DockerPruneResource resource)
        => resource switch
        {
            DockerPruneResource.All => ProtoPruneResource.All,
            DockerPruneResource.Volume => ProtoPruneResource.Volume,
            DockerPruneResource.Network => ProtoPruneResource.Network,
            DockerPruneResource.Image => ProtoPruneResource.Image,
            DockerPruneResource.Build => ProtoPruneResource.Build,
            _ => ProtoPruneResource.Unspecified
        };

    internal static PrunePlatformResult Map(this DockerPruneResult source)
        => new(
            source.Resource.Map(),
            source.SpaceReclaimed,
            source.VolumesDeleted,
            source.NetworksDeleted,
            source.ImagesDeleted,
            source.BuildCacheDeleted);

    internal static PrunePlatformResult Map(this PruneResponse source)
        => new(
            source.Resource.Map(),
            source.SpaceReclaimed,
            source.VolumesDeleted.ToList(),
            source.NetworksDeleted.ToList(),
            source.ImagesDeleted.ToList(),
            source.BuildCacheDeleted.ToList());

    internal static PruneResponse MapToProto(this DockerPruneResult source)
    {
        var response = new PruneResponse
        {
            Resource = source.Resource.MapToProto(),
            SpaceReclaimed = source.SpaceReclaimed
        };
        response.VolumesDeleted.AddRange(source.VolumesDeleted);
        response.NetworksDeleted.AddRange(source.NetworksDeleted);
        response.ImagesDeleted.AddRange(source.ImagesDeleted);
        response.BuildCacheDeleted.AddRange(source.BuildCacheDeleted);
        return response;
    }

    internal static DaemonEventInfo Map(this DaemonEventResult @event)
    {
        return @event switch
        {
            DaemonContainerResult containerEvent => containerEvent.Map(),
            DaemonImageResult imageEvent => imageEvent.Map(),
            DaemonVolumeResult volumeEvent => volumeEvent.Map(),
            DaemonNetworkResult networkEvent => networkEvent.Map(),
            _ => throw new NotSupportedException($"Event type {@event.GetType().Name} is not supported")
        };
    }

    internal static DaemonEventInfo Map(this DaemonContainerResult result)
    {
        return new DaemonContainerEventInfo
        (
            Action: result.Action,
            ContainerId: result.ContainerId,
            Container: result.Container?.Map()
        );
    }

    internal static DaemonEventInfo Map(this DaemonImageResult result)
    {
        return new DaemonImageEventInfo
        (
            Action: result.Action,
            ImageId: result.ImageId,
            Image: result.Image?.Map()
        );
    }

    internal static DaemonEventInfo Map(this DaemonVolumeResult result)
    {
        return new DaemonVolumeEventInfo
        (
            Action: result.Action,
            VolumeId: result.VolumeId,
            Volume: result.Volume?.Map()
        );
    }

    internal static DaemonEventInfo Map(this DaemonNetworkResult result)
    {
        return new DaemonNetworkEventInfo
        (
            Action: result.Action,
            NetworkId: result.NetworkId,
            Network: result.Network?.Map()
        );
    }

    internal static ContainerEventType Map(this Hosting.DockerClient.EventMessageType type)
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
