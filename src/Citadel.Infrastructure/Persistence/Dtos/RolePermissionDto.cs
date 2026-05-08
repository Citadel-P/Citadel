namespace Infrastructure.Persistence.Dtos;

internal sealed record RolePermissionDto(
    Guid Id,
    string Name,
    string RoleType,
    Guid? PermissionId,
    int? ResourceType,
    int? PermissionLevel,
    int? SpecificPermissions);
