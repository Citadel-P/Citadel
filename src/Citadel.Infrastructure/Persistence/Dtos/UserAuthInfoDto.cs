namespace Infrastructure.Persistence.Dtos;

internal sealed record UserAuthInfoDto(
    Guid Id,
    string Name, 
    string Email,
    string Password, 
    string RoleName, 
    int? PermissionCode);