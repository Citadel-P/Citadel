namespace Domain.Contracts.Resources.Identity;

public sealed record UserAuthInfo(
    Guid Id, 
    string Name, 
    string Email, 
    string? Password,
    IEnumerable<string> Roles,
    IEnumerable<AppPermission> Permissions);