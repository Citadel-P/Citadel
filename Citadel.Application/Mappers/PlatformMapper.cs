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

    internal static IEnumerable<PlatformStat> Map(this Dictionary<Guid, List<PlatformStatsResult>> stats)
    {
        foreach (var (platformId, platformStats) in stats)
        {
            foreach (var stat in platformStats)
            {
                yield return Map(stat.PlatformStat, platformId);
            }
        }
    }

    internal static PlatformStat Map(this DockerPlatformStat stat, Guid platformId)
       => new
       (
           created: stat.Created,
           platformId: platformId,
           memoryUsage: stat?.MemoryUsage ?? 0,
           cpuUsage: stat?.CpuUsage ?? 0,
           rxBytes: stat?.RxBytes ?? 0,
           txBytes: stat?.TxBytes ?? 0
       );
}
