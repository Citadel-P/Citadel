namespace Infrastructure.Persistence.Dtos;

internal sealed record RolePermissionDto(
    Guid Id,
    string Name,
    Guid? PermissionId,
    string? ResourceType,
    string? ResourceAction);
