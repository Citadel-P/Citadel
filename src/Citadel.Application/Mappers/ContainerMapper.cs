using Domain.Contracts.Resources.Containers;
using Domain.Entities;

namespace Application.Mappers;

internal static class ContainerMapper
{
    public static IEnumerable<Container> Map(this IEnumerable<DockerContainer> containers, Guid platformId)
        => containers.Select(s => s.Map(platformId));

    public static Container Map(this DockerContainer container, Guid platformId)
        => new
        (
            name: container.Name,
            image: container.Image,
            imageId: container.ImageId,
            stack: container.Stack,
            platformId: platformId,
            containerId: container.ContainerId,
            created: container.Created,
            state: container.State,
            ports: container.Ports
        );

    public static ContainerStat Map(this DockerContainerStat container, Guid containerId, long? created)
        => new
        (
            containerId: containerId,
            memoryActive: container.MemoryActive,
            memoryCache: container.MemoryCache,
            cpuUsage: container.CpuUsage,
            memoryLimit: container.MemoryLimit,
            rxBytes: container.RxBytes,
            txBytes: container.TxBytes,
            created: created ?? (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds
        );

    public static void Map(this DockerContainerStat container, ContainerStat destination, Guid containerId, long? created)
    {
        destination.ReInitialize(
            containerId: containerId,
            created: created ?? (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds,
            memoryActive: container.MemoryActive,
            memoryCache: container.MemoryCache,
            cpuUsage: container.CpuUsage,
            memoryLimit: container.MemoryLimit,
            rxBytes: container.RxBytes,
            txBytes: container.TxBytes
        );
    }
}
