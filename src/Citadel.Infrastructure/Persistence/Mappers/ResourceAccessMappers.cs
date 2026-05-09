using Domain.Entities.Identity;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class ResourceAccessMappers
{
    internal static IEnumerable<ResourceAccess> ToDomain(this IEnumerable<ResourceAccessDto> dtos)
        => dtos.Select(ToDomain);

    internal static ResourceAccess ToDomain(this ResourceAccessDto dto)
    {
        return ResourceAccess.FromPersistence(
            dto.Id,
            (ResourceType)dto.ResourceType,
            dto.ResourceId,
            dto.ActorId,
            (PermissionLevel)dto.PermissionLevel,
            Permission.FromSpecificPermissionsMask(dto.SpecificPermissions));
    }
}
