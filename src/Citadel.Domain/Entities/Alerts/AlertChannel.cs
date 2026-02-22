namespace Domain.Entities.Alerts;

public class AlertChannel : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public AlertDestination AlertDestination { get; private set; }
    public string Url { get; private set; }
    public bool IsActive { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public AlertChannel(
        AlertDestination alertDestination,
        string url,
        bool isActive,
        Guid actorId
        )
    {
        Url = url;
        IsActive = isActive;
        CreatedByActorId = actorId;
        CreatedAt = DateTime.UtcNow;
        AlertDestination = alertDestination;
    }

    public static AlertChannel FromPersistence(
        Guid id,
        AlertDestination alertDestination,
        string url,
        bool isActive,
        Guid actorId)
    => new (alertDestination, url, isActive, actorId) { Id = id };
}
