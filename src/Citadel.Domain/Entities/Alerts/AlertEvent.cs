namespace Domain.Entities.Alerts;

public sealed class AlertEvent
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid AlertRuleId { get; private set; }
    public AlertType Type { get; private set; }
    public AlertSeverity Severity { get; private set; }
    public AlertEventInfo Info { get; private set; }
    public Guid? ResourceId { get; private set; }
    public AlertResourceType ResourceType { get; private set; }
    public DateTime CreatedAt { get; private set; }

    public AlertEvent(
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertEventInfo info,
        Guid resourceId,
        AlertResourceType resourceType)
    {
        if (!AlertTypeMetadata.IsValidInfo(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        AlertRuleId = alertRuleId;
        Type = type;
        Severity = severity;
        Info = info;
        ResourceId = resourceId;
        ResourceType = resourceType;
        CreatedAt = DateTime.UtcNow;
    }

    public static AlertEvent FromPersistence(
        Guid id,
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertEventInfo info,
        Guid resourceId,
        AlertResourceType resourceType,
        DateTime createdAt)
    {
        if (!AlertTypeMetadata.IsValidInfo(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        return new AlertEvent(
            alertRuleId: alertRuleId,
            type: type,
            severity: severity,
            info: info,
            resourceId: resourceId,
            resourceType: resourceType)
        {
            Id = id,
            CreatedAt = createdAt
        };
    }
}
