using Domain.Entities;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class ContainerMappers
{
    internal static IEnumerable<Container> ToDomain(this IEnumerable<ContainerDto> containers)
        => containers.Select(ToDomain);

    internal static Container ToDomain(this ContainerDto container)
    {
        return
        Container.FromPersistence(
            id: container.Id,
            platformId: container.PlatformId,
            containerId: container.ContainerId,
            name: container.Name,
            image: container.Image,
            created: container.Created,
            updated: container.Updated,
            state: container.State,
            ports: container.Ports,
            stack: container.Stack,
            platform: container.Platform?.ToDomain(),
            stats: container.Stats?.Select(ToDomain).ToList());
    }

    internal static IEnumerable<ContainerStat> ToDomain(this IEnumerable<ContainerStatDto> stats)
        => stats.Select(ToDomain);

    internal static ContainerStat ToDomain(this ContainerStatDto stat)
    {
        return
        ContainerStat.FromPersistence(
            id: stat.Id,
            containerId: stat.ContainerId,
            created: stat.Created,
            memoryUsage: stat.MemoryUsage,
            cpuUsage: stat.CpuUsage,
            memoryLimit: stat.MemoryLimit,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes);
    }
}

