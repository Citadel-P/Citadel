using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;

namespace Application.Mappers;

internal static class PlatformMapper
{
    internal static Platform Map(this PlatformResult platformInfo, string platformAddress, string platformName, PlatformConnectorType type)
    {
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
            platformDescriptor: platformInfo.Descriptor);
    }

    public static void Map(this PlatformStatsResult stat, PlatformStat destination, Guid platformId)
       => destination.ReInitialize(
           platformId: platformId,
           rxBytes: stat?.PlatformStat.RxBytes ?? 0,
           txBytes: stat?.PlatformStat.TxBytes ?? 0,
           cpuUsage: stat?.PlatformStat.CpuUsage ?? 0,
           memoryUsage: stat?.PlatformStat.MemoryUsage ?? 0,
           created: stat?.PlatformStat.Created ?? (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds
       );
}
