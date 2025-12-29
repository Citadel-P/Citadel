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
            platformId: dto.PlatformId,
            description: dto.Description,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt,
            status: Enum.Parse<DeploymentStatus>(dto.Status),
            updateBehavior: Enum.Parse<UpdateBehavior>(dto.UpdateBehavior),
            autoUpdateState: new AutoUpdateState(
                LastCheckedAt: dto.AutoUpdateState_LastCheckedAt,
                Status: Enum.Parse<AutoUpdateStatus>(dto.AutoUpdateState_Status),
                CurrentDigest: dto.AutoUpdateState_CurrentDigest,
                RemoteDigest: dto.AutoUpdateState_RemoteDigest,
                LastError: dto.AutoUpdateState_LastError),
            platform: dto.Platform_Name == null ? null : Platform.FromPersistence(id: dto.PlatformId, name: dto.Platform_Name, address: string.Empty, 
                networkCount:0, volumeCount: 0, imageCount: 0, cpuCount: 0, memTotal: 0, status: Enum.Parse<PlatformStatus>(dto.Platform_Status), connectorType: PlatformConnectorType.Unknown, platformDescriptor: null),
            image: dto.Image_Id == null ? null : Image.FromPersistence(id: dto.Image_Id.Value, name: dto.Image_Name, tags: [], dockerImageId: string.Empty, size: 0, containers: 0, platformId: Guid.Empty, createdAt: DateTime.MinValue),
            spec: dto.Spec == null ? null : JsonSerializer.Deserialize(dto.Spec, DeploymentJsonContext.Default.DeploymentSpec));
    }
}
