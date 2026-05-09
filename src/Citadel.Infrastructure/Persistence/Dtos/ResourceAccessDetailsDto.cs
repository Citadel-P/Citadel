namespace Infrastructure.Persistence.Dtos;

internal sealed record ResourceAccessDetailsDto(
    Guid Id,
    Guid ActorId,
    Guid ResourceId,
    int ResourceType,
    string? ResourceName,
    int PermissionLevel,
    int SpecificPermissions);
