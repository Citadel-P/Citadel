namespace Infrastructure.Persistence.Dtos;

internal sealed record UserAuthInfoDto(
    Guid Id,
    Guid ActorId,
    string Name, 
    string Email,
    string Password, 
    string? RoleName,
    string? PermissionName);
