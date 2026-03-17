using System.Security.Cryptography;

namespace Domain.Entities.Alerts;

public sealed class AlertEvent : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid AlertRuleId { get; private set; }
    public AlertType Type { get; private set; }
    public AlertSeverity Severity { get; private set; }
    public AlertEventInfo Info { get; private set; }
    public Guid? ResourceId { get; private set; }
    public AlertResourceType ResourceType { get; private set; }
    public string DeduplicationKey { get; private set; }
    public string? OpenIncidentKey { get; private set; }
    public Guid? AcknowledgedByActorId { get; private set; }
    public DateTime? AcknowledgedAt { get; private set; }
    public Guid? ResolvedByActorId { get; private set; }
    public DateTime? ResolvedAt { get; private set; }
    public string? ResolutionNote { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public DateTime UpdatedAt { get; private set; }

    public AlertEventStatus Status
    {
        get
        {
            if (ResolvedAt is not null)
                return AlertEventStatus.Resolved;

            if (AcknowledgedAt is not null)
                return AlertEventStatus.Acknowledged;

            return AlertEventStatus.Active;
        }
    }

    public AlertEvent(
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertEventInfo info,
        Guid resourceId,
        AlertResourceType resourceType,
        Guid createdByActorId)
    {
        if (!AlertTypeMetadata.IsValidInfo(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        if (createdByActorId == Guid.Empty)
            throw new ArgumentException("CreatedByActorId is required.", nameof(createdByActorId));

        AlertRuleId = alertRuleId;
        Type = type;
        Severity = severity;
        Info = info;
        ResourceId = resourceId;
        ResourceType = resourceType;
        CreatedByActorId = createdByActorId;
        DeduplicationKey = BuildDeduplicationKey(alertRuleId, resourceId, resourceType);
        OpenIncidentKey = DeduplicationKey;
        CreatedAt = DateTime.UtcNow;
        UpdatedAt = CreatedAt;
    }

    public void Acknowledge(Guid actorId, DateTime utcNow)
    {
        if (AcknowledgedAt is not null)
            throw new InvalidOperationException("Alert already acknowledged.");

        AcknowledgedByActorId = actorId;
        AcknowledgedAt = utcNow;
        UpdatedAt = utcNow;
    }

    public void Resolve(Guid actorId, DateTime utcNow, string? resolutionNote = null)
    {
        if (ResolvedAt is not null)
            throw new InvalidOperationException("Alert already resolved.");

        if (AcknowledgedAt is null)
        {
            AcknowledgedByActorId = actorId;
            AcknowledgedAt = utcNow;
        }

        ResolvedByActorId = actorId;
        ResolvedAt = utcNow;
        ResolutionNote = resolutionNote;
        OpenIncidentKey = null;
        UpdatedAt = utcNow;
    }

    public static AlertEvent FromPersistence(
        Guid id,
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertEventInfo info,
        Guid resourceId,
        AlertResourceType resourceType,
        string deduplicationKey,
        string? openIncidentKey,
        Guid? acknowledgedByActorId,
        DateTime? acknowledgedAt,
        Guid? resolvedByActorId,
        DateTime? resolvedAt,
        string? resolutionNote,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt)
    {
        if (!AlertTypeMetadata.IsValidInfo(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        var alertEvent = new AlertEvent(
            alertRuleId: alertRuleId,
            type: type,
            severity: severity,
            info: info,
            resourceId: resourceId,
            resourceType: resourceType,
            createdByActorId: createdByActorId)
        {
            Id = id,
            DeduplicationKey = deduplicationKey,
            OpenIncidentKey = openIncidentKey,
            AcknowledgedByActorId = acknowledgedByActorId,
            AcknowledgedAt = acknowledgedAt,
            ResolvedByActorId = resolvedByActorId,
            ResolvedAt = resolvedAt,
            ResolutionNote = resolutionNote,
            UpdatedAt = updatedAt
        };

        alertEvent.CreatedAt = createdAt;
        alertEvent.CreatedByActorId = createdByActorId;

        return alertEvent;
    }

    private static string BuildDeduplicationKey(
        Guid alertRuleId,
        Guid resourceId,
        AlertResourceType resourceType)
    {
        Span<byte> buffer = stackalloc byte[16 + 16 + 4];

        alertRuleId.TryWriteBytes(buffer);
        resourceId.TryWriteBytes(buffer[16..]);
        BitConverter.TryWriteBytes(buffer[32..], (int)resourceType);

        Span<byte> hash = stackalloc byte[32];
        SHA256.TryHashData(buffer, hash, out _);

        return Convert.ToHexString(hash);
    }
}
