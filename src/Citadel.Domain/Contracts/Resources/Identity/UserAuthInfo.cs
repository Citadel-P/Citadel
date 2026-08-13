namespace Domain.Contracts.Resources.Identity;

public sealed record UserAuthInfo(
    Guid Id,
    Guid ActorId,
    string Name, 
    string Email, 
    string? Password,
    IEnumerable<string> Roles);

public sealed record RunAsActorInfo(
    Guid ActorId,
    Guid PrincipalId,
    ActorType Type,
    string Name,
    bool IsEnabled,
    bool IsArchived,
    string[] Roles);
