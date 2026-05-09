namespace Domain.Contracts.Resources.Identity;

public sealed record UserDetails(
    Guid Id,
    string Name,
    string Email,
    Guid ActorId,
    bool IsEnabled,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    IEnumerable<ResourceInfo>? Teams = null,
    IEnumerable<ResourceInfo>? Roles = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null);
