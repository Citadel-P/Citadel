using Domain.Contracts.Resources.Containers;
using Domain.Entities;

namespace Application.Mappers;

internal static class ContainerMapper
{
    internal static IEnumerable<Container> Map(this IEnumerable<DockerContainer> containers, IEnumerable<Image> images, Guid platformId)
    {
        foreach (var container in containers)
        {
            var image = images.FirstOrDefault(i => i.DockerImageId == container.ImageId && i.PlatformId == platformId);
            yield return container.Map(platformId, image?.Id);
        }
    }

    internal static Container Map(this DockerContainer container, Guid platformId, Guid? imageId)
        => new
        (
            name: container.Name,
            dockerImageId: container.ImageId,
            dockerStack: container.Stack,
            platformId: platformId,
            dockerContainerId: container.Id,
            created: container.Created,
            state: container.State,
            ports: container.Ports,
            imageId: imageId,
            stackId: container.StackId,
            isSystem: container.IsSystem,
            systemRole: container.SystemRole,
            hasCitadelOwnershipLabels: container.HasCitadelOwnershipLabels
        );

    internal static ContainerStat Map(this DockerContainerStat container, Guid containerId, long? created)
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
