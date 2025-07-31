using System.Text.Json;
using Domain;
using Domain.Contracts.Resources.Containers;
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
            id: Guid.Parse(container.Id),
            platformId: Guid.Parse(container.PlatformId),
            containerId: container.ContainerId,
            name: container.Name,
            image: container.Image,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IReadOnlyCollectionContainerPort) ?? [],
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
            id: stat.Id != null ? Guid.Parse(stat.Id) : Guid.Empty,
            containerId: stat.ContainerId != null ? Guid.Parse(stat.ContainerId) : Guid.Empty,
            created: stat.Created,
            memoryActive: stat.MemoryActive,
            memoryCache: stat.MemoryCache,
            cpuUsage: stat.CpuUsage,
            memoryLimit: stat.MemoryLimit,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes);
    }

    internal static ContainerInfo Map(this ContainerInfoDto container)
    {
        return new ContainerInfo(
            Id: container.Id,
            Name: container.Name,
            PlatformId: container.PlatformId,
            ContainerId: container.ContainerId,
            PlatformName: container.PlatformName,
            State: Enum.Parse<ContainerStateStatus>(container.State));
    }
}

