namespace Infrastructure.Persistence.Dtos;

internal sealed record ResourceAccessDto(
        Guid Id,
        Guid ResourceId,
        Guid ActorId,
        int ResourceType,
        int PermissionLevel,
        int SpecificPermissions);
