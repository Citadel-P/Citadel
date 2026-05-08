namespace Domain.Contracts.Resources.Identity;

public sealed record UserAuthInfo(
    Guid Id,
    Guid ActorId,
    string Name, 
    string Email, 
    string? Password,
    IEnumerable<string> Roles);
