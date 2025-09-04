using System.Text.Json;
using Domain;
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
            imageId: container.ImageId,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            platform: container.Platform?.ToDomain(),
            stats: container.Stats?.Select(ToDomain).ToList());
    }

    internal static IEnumerable<Container> ToDomain(this IEnumerable<ContainerWithLastStatDto> containers)
        => containers.Select(ToDomain);

    internal static Container ToDomain(this ContainerWithLastStatDto container)
    {
        return
        Container.FromPersistence(
            id: container.Id,
            platformId: container.PlatformId,
            containerId: container.ContainerId,
            name: container.Name,
            image: container.Image,
            imageId: container.ImageId,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            platform: container.Platform?.ToDomain(),
            stats: [new ContainerStat(
                containerId: container.Id,
                memoryActive: container.Stat_MemoryActive,
                memoryCache: container.Stat_MemoryCache,
                cpuUsage: container.Stat_CpuUsage,
                memoryLimit: container.Stat_MemoryLimit,
                rxBytes: container.Stat_RxBytes,
                txBytes: container.Stat_TxBytes,
                created: container.Stat_Created
                )]);
    }

    internal static IEnumerable<ContainerStat> ToDomain(this IEnumerable<ContainerStatDto> stats)
        => stats.Select(ToDomain);

    internal static ContainerStat ToDomain(this ContainerStatDto stat)
    {
        return
        ContainerStat.FromPersistence(
            id: stat.Id != null ? stat.Id : Guid.Empty,
            containerId: stat.ContainerId != null ? stat.ContainerId : Guid.Empty,
            created: stat.Created,
            memoryActive: stat.MemoryActive,
            memoryCache: stat.MemoryCache,
            cpuUsage: stat.CpuUsage,
            memoryLimit: stat.MemoryLimit,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes);
    }

}

