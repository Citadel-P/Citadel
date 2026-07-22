namespace Domain.Entities.Platforms;

public sealed record EdgeAgentEnrollment(
    Guid Id,
    Guid PlatformId,
    EdgeAgentResourceType ResourceType,
    Guid ResourceId,
    string TokenHash,
    DateTime ExpiresAtUtc,
    DateTime? UsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid CreatedByActorId,
    DateTime CreatedAtUtc)
{
    public EdgeAgentResourceType NormalizedResourceType => ResourceType;
    public Guid NormalizedResourceId => ResourceId == Guid.Empty ? PlatformId : ResourceId;

    public bool IsActive(DateTime utcNow)
        => UsedAtUtc is null && RevokedAtUtc is null && ExpiresAtUtc > utcNow;
}
