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
            spec: JsonSerializer.Deserialize(dto.Spec, DeploymentJsonContext.Default.DeploymentSpec)
                ?? throw new NotImplementedException($"Deployment spec is missing for deployment id {dto.Id}"));
    }
}
