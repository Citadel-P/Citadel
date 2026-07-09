namespace Domain.Entities.Platforms;

public sealed record EdgeAgentEnrollment(
    Guid Id,
    Guid PlatformId,
    string TokenHash,
    DateTime ExpiresAtUtc,
    DateTime? UsedAtUtc,
    DateTime? RevokedAtUtc,
    Guid CreatedByActorId,
    DateTime CreatedAtUtc)
{
    public bool IsActive(DateTime utcNow)
        => UsedAtUtc is null && RevokedAtUtc is null && ExpiresAtUtc > utcNow;
}
