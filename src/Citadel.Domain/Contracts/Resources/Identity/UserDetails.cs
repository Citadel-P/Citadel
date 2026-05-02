namespace Domain.Contracts.Resources.Identity;

public sealed record UserDetails(
    Guid Id,
    string Name,
    string Email,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    IEnumerable<string>? Teams = null,
    IEnumerable<string>? Roles = null);
