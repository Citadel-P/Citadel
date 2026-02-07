using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class DeploymentMappers
{
    internal static IEnumerable<Deployment> ToDomain(this IEnumerable<DeploymentDto> dtos)
        => dtos.Select(ToDomain);

    internal static Deployment ToDomain(this DeploymentDto dto)
    {
        return Deployment.FromPersistence(
            id: dto.Id,
            name: dto.Name,
            rowVersion: dto.RowVersion,
            platformId: dto.PlatformId,
            description: dto.Description,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt,
            controlStartedAt: dto.ControlStartedAt,
            status: Enum.Parse<DeploymentStatus>(dto.Status),
            updateBehavior: Enum.Parse<UpdateBehavior>(dto.UpdateBehavior),
            controlState: Enum.Parse<ResourceControlState>(dto.ControlState),
            controlTriggeredBy: dto.ControlTriggeredBy,
            autoUpdateState: new AutoUpdateState(
                LastCheckedAt: dto.AutoUpdateState_LastCheckedAt,
                Status: Enum.Parse<AutoUpdateStatus>(dto.AutoUpdateState_Status),
                CurrentDigest: dto.AutoUpdateState_CurrentDigest,
                RemoteDigest: dto.AutoUpdateState_RemoteDigest,
                LastError: dto.AutoUpdateState_LastError),
            container: dto.Container_ContainerId == null ? null : Container.FromPersistence(
                id: dto.Container_ContainerId.Value,
                platformId: dto.PlatformId,
                dockerImageId: string.Empty,
                name: string.Empty,
                created: 0,
                updated: 0,
                rowVersion: 0,
                controlStartedAt: null,
                controlTriggeredBy: null,
                controlState: ResourceControlState.Idle,
                ports: null,
                state: ContainerStateStatus.Unknown,
                dockerContainerId: dto.Container_DockerContainerId!,
                deploymentId: dto.Id),
            platform: dto.Platform_Name == null ? null : Platform.FromPersistence(id: dto.PlatformId, name: dto.Platform_Name, address: string.Empty, 
                networkCount:0, volumeCount: 0, imageCount: 0, cpuCount: 0, memTotal: 0, status: dto.Platform_Status != null ? Enum.Parse<PlatformStatus>(dto.Platform_Status) : PlatformStatus.Offline, connectorType: PlatformConnectorType.Unknown, platformDescriptor: null),
            image: dto.Image_Id == null ? null : Image.FromPersistence(id: dto.Image_Id.Value, name: dto.Image_Name, tags: [], dockerImageId: string.Empty, size: 0, containers: 0, platformId: Guid.Empty, createdAt: DateTime.MinValue, rowVersion:0, controlStartedAt: null, controlState: ResourceControlState.Idle),
            spec: dto.Spec == null ? null : JsonSerializer.Deserialize(dto.Spec, DeploymentJsonContext.Default.DeploymentSpec));
    }
}
