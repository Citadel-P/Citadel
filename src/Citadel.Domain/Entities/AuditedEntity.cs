namespace Domain.Entities;

public abstract class AuditedEntity(Guid createdByActorId, DateTime? createdAt = null)
{
    public DateTime CreatedAt { get; protected set; } = createdAt ?? DateTime.UtcNow;
    public Guid CreatedByActorId { get; protected set; } = createdByActorId;
}
