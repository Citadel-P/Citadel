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
            url: dto.Url,
            created: dto.Created,
            type: dto.Type,
            configuration: dto.Configuration);
    }
}
