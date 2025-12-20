using System.Text.Json;
using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class RegistryMappers
{
    internal static IEnumerable<Registry > ToDomain(this IEnumerable<RegistryDto> dtos)
        => dtos.Select(ToDomain);

    internal static Registry ToDomain(this RegistryDto dto)
    {
        return Registry.FromPersistence(
            id: dto.Id,
            name: dto.Name,
            description: dto.Description,
            registryHost: dto.RegistryHost,
            createdByActorId: dto.CreatedByActorId,
            createdAt: dto.CreatedAt,
            status: Enum.Parse<RegistryStatus>(dto.Status),
            configuration: JsonSerializer.Deserialize(dto.Configuration, RegistryJsonContext.Default.RegistryConfigurationBase)
                ?? throw new NotImplementedException($"Registry configuration is missing for registry id {dto.Id}"));
    }
}
