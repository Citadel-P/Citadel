namespace Domain.Entities;

internal interface IAuditedEntity
{
    DateTime CreatedAt { get;  }
    Guid CreatedByActorId { get; }
}
