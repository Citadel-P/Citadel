namespace Domain.Entities;

public interface IAuditedEntity
{
    DateTime CreatedAt { get;  }
    Guid CreatedByActorId { get; }
}
