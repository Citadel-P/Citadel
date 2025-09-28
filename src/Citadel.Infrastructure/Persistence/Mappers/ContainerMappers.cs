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
            imageEntityId: container.ImageEntityId,
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
            imageEntityId: container.ImageEntityId,
            stats: [new ContainerStat(
                ContainerId: container.Id,
                MemoryActive: container.Stat_MemoryActive,
                MemoryCache: container.Stat_MemoryCache,
                CpuUsage: container.Stat_CpuUsage,
                MemoryLimit: container.Stat_MemoryLimit,
                RxBytes: container.Stat_RxBytes,
                TxBytes: container.Stat_TxBytes,
                Created: container.Stat_Created
                )]);
    }

    internal static IEnumerable<ContainerStat> ToDomain(this IEnumerable<ContainerStatDto> stats)
        => stats.Select(ToDomain);

    internal static ContainerStat ToDomain(this ContainerStatDto stat)
        => new (
            ContainerId: stat.ContainerId != Guid.Empty ? stat.ContainerId : Guid.Empty,
            Created: stat.Created,
            MemoryActive: stat.MemoryActive,
            MemoryCache: stat.MemoryCache,
            CpuUsage: stat.CpuUsage,
            MemoryLimit: stat.MemoryLimit,
            RxBytes: stat.RxBytes,
            TxBytes: stat.TxBytes);

}

