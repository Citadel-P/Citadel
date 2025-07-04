using Domain.Entities;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class PlatformMappers
{
    internal static IEnumerable<Platform> ToDomain(this IEnumerable<PlatformDto> platforms)
        => platforms.Select(ToDomain);
    
    internal static Platform ToDomain(this PlatformDto platform)
    {
        return
        Platform.FromPersistence(
            id: platform.Id,
            name: platform.Name,
            address: platform.Address,
            networkCount: platform.NetworkCount,
            volumeCount: platform.VolumeCount,
            imageCount: platform.ImageCount,
            cpuCount: platform.CpuCount,
            memTotal: platform.MemTotal,
            status: platform.Status,
            connectorType: platform.ConnectorType,
            platformDescriptor: platform.PlatformDescriptor,
            serverVersion: platform.ServerVersion,
            agentVersion: platform.AgentVersion,
            stats: platform.Stats?.Select(ToDomain).ToList()
            );
    }

    internal static IEnumerable<PlatformStat> ToDomain(this IEnumerable<PlatformStatDto> stats)
        => stats.Select(ToDomain);

    internal static PlatformStat ToDomain(this PlatformStatDto stat)
    {
        return
        PlatformStat.FromPersistence(
            id: stat.Id,
            platformId: stat.PlatformId,
            created: stat.Created,
            cpuUsage: stat.CpuUsage ?? 0,
            memoryUsage: stat.MemoryUsage ?? 0,
            rxBytes: stat.RxBytes ?? 0,
            txBytes: stat.TxBytes ?? 0);
    }
}



