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
            stack: container.Stack,
            platformId: platformId,
            containerId: container.ContainerId,
            created: container.Created,
            command: container.Command,
            state: container.State,
            ports: container.Ports?.ToList() ?? []
        );

    public static ContainerStat Map(this DockerContainerStat container, Guid containerId, long? created)
        => new
        (
            containerId: containerId,
            memoryUsage: container.MemoryUsage,
            cpuUsage: container.CpuUsage,
            memoryLimit: container.MemoryLimit,
            rxBytes: container.RxBytes,
            txBytes: container.TxBytes,
            created: created ?? DateTimeOffset.UtcNow.ToUnixTimeSeconds()
        );
}
