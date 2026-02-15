using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

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
            deploymentId: container.DeploymentId,
            rowVersion: container.RowVersion,
            controlStartedAt: container.ControlStartedAt,
            controlTriggeredBy: container.ControlTriggeredBy,
            controlState: Enum.Parse<ResourceControlState>(container.ControlState),
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
            rowVersion: container.RowVersion,
            controlStartedAt: container.ControlStartedAt,
            controlTriggeredBy: container.ControlTriggeredBy,
            controlState: Enum.Parse<ResourceControlState>(container.ControlState),
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            imageId: container.ImageId,
            deploymentId: container.DeploymentId,
            deployment: container.Deployment_DeploymentId == null ? null : Deployment.FromPersistence
            (
                id: container.Deployment_DeploymentId ?? Guid.Empty,
                name: container.Deployment_DeploymentName,
                platformId: container.PlatformId,
                status: Enum.Parse<DeploymentStatus>(container.Deployment_DeploymentStatus ?? DeploymentStatus.Unknown.ToString()),
                createdAt: DateTime.MinValue,
                createdByActorId: Guid.Empty,
                rowVersion:0,
                controlState: ResourceControlState.Idle,
                controlStartedAt: 0,
                controlTriggeredBy: null
            ),
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
                registryId: container.Image_RegistryId,
                rowVersion: 0,
                controlStartedAt: null,
                controlState: ResourceControlState.Idle
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
            rowVersion: container.RowVersion,
            controlStartedAt: container.ControlStartedAt,
            controlTriggeredBy: container.ControlTriggeredBy,
            controlState: Enum.Parse<ResourceControlState>(container.ControlState),
            state: Enum.Parse<ContainerStateStatus>(container.State),
            ports: JsonSerializer.Deserialize(container.Ports, ContainerPortsContext.Default.IDictionaryStringIReadOnlyListHostPortBinding) ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            stack: container.Stack,
            imageId: container.ImageId,
            deploymentId: container.DeploymentId,
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
                registryId: container.Image_RegistryId,
                rowVersion: 0,
                controlStartedAt: null,
                controlState: ResourceControlState.Idle
            ),
            deployment: container.Deployment_DeploymentId == null ? null : Deployment.FromPersistence
            (
                id: container.Deployment_DeploymentId ?? Guid.Empty,
                name: container.Deployment_DeploymentName,
                platformId: container.PlatformId,
                status: Enum.Parse<DeploymentStatus>(container.Deployment_DeploymentStatus ?? DeploymentStatus.Unknown.ToString()),
                createdAt: DateTime.MinValue,
                createdByActorId: Guid.Empty,
                rowVersion: 0,
                controlState: ResourceControlState.Idle,
                controlStartedAt: 0,
                controlTriggeredBy: null
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

