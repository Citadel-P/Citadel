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
            ContainerId: containerId,
            MemoryActive: container.MemoryActive,
            MemoryCache: container.MemoryCache,
            CpuUsage: container.CpuUsage,
            MemoryLimit: container.MemoryLimit,
            RxBytes: container.RxBytes,
            TxBytes: container.TxBytes,
            Created: created ?? (long)(DateTime.UtcNow - DateTime.UnixEpoch).TotalSeconds
        );

}
