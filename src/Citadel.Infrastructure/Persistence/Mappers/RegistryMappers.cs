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
            registryHost: dto.RegistryHost,
            created: DateTime.Parse(dto.Created),
            configuration: JsonSerializer.Deserialize(dto.Configuration, RegistryJsonContext.Default.RegistryConfigurationBase)
                ?? throw new NotImplementedException($"Registry configuration is missing for registry id {dto.Id}"));
    }
}
