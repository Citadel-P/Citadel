namespace Infrastructure.Persistence.Dtos;

internal sealed record PermissionAssignmentDto(
    int ResourceType,
    int PermissionLevel,
    int SpecificPermissions);
