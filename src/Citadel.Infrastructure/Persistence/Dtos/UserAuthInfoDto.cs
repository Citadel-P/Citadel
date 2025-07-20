namespace Infrastructure.Persistence.Dtos;

internal sealed record UserAuthInfoDto(
    string Id, // Guid
    string Name, 
    string Email,
    string Password, 
    string RoleName, 
    int? PermissionCode);