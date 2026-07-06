namespace Infrastructure.Persistence.Dtos;

internal sealed record UserAuthInfoDto(
    Guid Id,
    string Name, 
    string Email,
    Guid ActorId,
    string Password, 
    string? RoleName,
    int? PermissionResourceType,
    int? PermissionLevel,
    int? SpecificPermissions);
