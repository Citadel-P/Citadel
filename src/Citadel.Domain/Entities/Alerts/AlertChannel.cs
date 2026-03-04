using System.Text.Json.Serialization;

namespace Domain.Entities.Alerts;

public class AlertChannel : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; }
    public AlertDestination AlertDestination { get; private set; }
    public string Url { get; private set; }
    public bool IsActive { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    [JsonConstructor]
    public AlertChannel(
        string? name,
        AlertDestination alertDestination,
        string url,
        bool isActive,
        Guid createdByActorId
        )
    {
        Name = string.IsNullOrWhiteSpace(name) ? alertDestination.ToString() : name;
        Url = url;
        IsActive = isActive;
        CreatedByActorId = createdByActorId;
        CreatedAt = DateTime.UtcNow;
        AlertDestination = alertDestination;
    }

    public AlertChannel(
        AlertDestination alertDestination,
        string url,
        bool isActive,
        Guid createdByActorId)
        : this(null, alertDestination, url, isActive, createdByActorId)
    {
    }

    public static AlertChannel FromPersistence(
        Guid id,
        string name,
        AlertDestination alertDestination,
        string url,
        bool isActive,
        Guid actorId)
    => new (name, alertDestination, url, isActive, actorId) { Id = id };

    public void PartialUpdate(
        string? name = null,
        AlertDestination? alertDestination = null,
        string? url = null,
        bool? isActive = null)
    {
        if (name is not null)
            Name = name;

        if (alertDestination is not null)
            AlertDestination = alertDestination.Value;

        if (url is not null)
            Url = url;

        if (isActive is not null)
            IsActive = isActive.Value;
    }
}
