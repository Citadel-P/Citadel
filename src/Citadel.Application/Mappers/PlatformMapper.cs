using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;

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

    internal static List<PlatformStat> Map(this Dictionary<Guid, List<PlatformStatsResult>> stats)
    {
        var result = new List<PlatformStat>();
        foreach (var (platformId, statList) in stats)
        {
            foreach (var stat in statList)
            {
                var platformStat = stat.Map(platformId);
                result.Add(platformStat);
            }
        }
        return result;
    }

    internal static PlatformStat Map(this PlatformStatsResult stat, Guid platformId)
       => new(
           PlatformId: platformId,
           RxBytes: stat?.PlatformStat.RxBytes ?? 0,
           TxBytes: stat?.PlatformStat.TxBytes ?? 0,
           CpuUsage: stat?.PlatformStat.CpuUsage ?? 0,
           MemoryUsage: stat?.PlatformStat.MemoryUsage ?? 0,
           Created: stat?.PlatformStat.Created ?? (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds,
           DiskUsedBytes: stat?.PlatformStat.DiskUsedBytes,
           DiskTotalBytes: stat?.PlatformStat.DiskTotalBytes,
           DiskUsage: stat?.PlatformStat.DiskUsage
       );
}
