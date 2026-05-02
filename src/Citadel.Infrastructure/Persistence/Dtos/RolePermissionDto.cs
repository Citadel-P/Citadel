namespace Infrastructure.Persistence.Dtos;

internal sealed record RolePermissionDto(
    Guid Id,
    string Name,
    string RoleType,
    Guid? PermissionId,
    string? ResourceType,
    string? ResourceAction);
