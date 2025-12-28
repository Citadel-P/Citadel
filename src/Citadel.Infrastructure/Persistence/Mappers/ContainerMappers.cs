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
            dockerContainerId: container.DockerContainerId,
            name: container.Name,
            dockerImageId: container.DockerImageId,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            imageId: container.ImageId,
            stats: container.Stats?.Select(ToDomain).ToList());
    }

    internal static Container? ToDomain(this ContainerWithImageDto? container)
    {
        return
        Container.FromPersistence(
            id: container.Id,
            platformId: container.PlatformId,
            dockerContainerId: container.DockerContainerId,
            name: container.Name,
            dockerImageId: container.DockerImageId,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            imageId: container.ImageId,
            image: container.Image_ImageId != null ? Image.FromPersistence(
                id: container.Image_ImageId ?? Guid.Empty,
                name: container.Image_Name,
                tags: string.IsNullOrEmpty(container?.Image_Tags) ? [] : JsonSerializer.Deserialize(container.Image_Tags, ImagTagsContext.Default.IEnumerableString),
                dockerImageId: container.Image_DockerImageId,
                size: container.Image_Size ?? 0,
                containers: container.Image_Containers ?? 0,
                platformId: container.Image_platformId ?? Guid.Empty,
                createdAt: container.Image_CreatedAt ?? DateTime.MinValue,
                updatedAt: container.Image_UpdatedAt,
                registryId: container.Image_RegistryId
                ) : null);
    }

    internal static IEnumerable<Container> ToDomain(this IEnumerable<ContainerWithLastStatDto> containers)
        => containers.Select(ToDomain);

    internal static Container ToDomain(this ContainerWithLastStatDto container)
    {
        return
        Container.FromPersistence(
            id: container.Id,
            platformId: container.PlatformId,
            dockerContainerId: container.DockerContainerId,
            name: container.Name,
            dockerImageId: container.DockerImageId,
            created: container.Created,
            updated: container.Updated,
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            imageId: container.ImageId,
            image: container.Image_ImageId == null ? null : Image.FromPersistence
            (
                id: container.Image_ImageId ?? Guid.Empty,
                name: container.Image_Name,
                tags: string.IsNullOrEmpty(container?.Image_Tags) ? [] : JsonSerializer.Deserialize(container.Image_Tags, ImagTagsContext.Default.IEnumerableString),
                dockerImageId: container.Image_DockerImageId,
                size: container.Image_Size ?? 0,
                containers: container.Image_Containers ?? 0,
                platformId: container.Image_platformId ?? Guid.Empty,
                createdAt: container.Image_CreatedAt ?? DateTime.MinValue,
                updatedAt: container.Image_UpdatedAt,
                registryId: container.Image_RegistryId
            ),
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

