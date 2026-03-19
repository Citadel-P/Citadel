using Hosting.Common.ErrorTypes;
using LightResults;
using System.Buffers;
using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;

namespace Domain.Entities.Alerts;

public sealed class AlertEvent
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid AlertRuleId { get; private set; }
    public AlertType Type { get; private set; }
    public AlertSeverity Severity { get; private set; }
    public AlertEventInfo Info { get; private set; }
    public Guid? ResourceId { get; private set; }
    public string ResourceName { get; private set; }
    public AlertResourceType ResourceType { get; private set; }
    public string DeduplicationKey { get; private set; }
    public string? OpenIncidentKey { get; private set; }
    public Guid? AcknowledgedByActorId { get; private set; }
    public DateTime? AcknowledgedAt { get; private set; }
    public Guid? ResolvedByActorId { get; private set; }
    public DateTime? ResolvedAt { get; private set; }
    public string? ResolutionNote { get; private set; }

    public DateTime CreatedAt { get; private set; }

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
        string resourceName,
        AlertResourceType resourceType,
        string? deduplicationComponent = null)
    {
        if (!AlertTypeMetadata.IsValidInfo(type, info))
            throw new ArgumentException("AlertInfo does not match AlertType.");

        if (string.IsNullOrWhiteSpace(resourceName))
            throw new ArgumentException("ResourceName is required.", nameof(resourceName));

        AlertRuleId = alertRuleId;
        Type = type;
        Severity = severity;
        Info = info;
        ResourceId = resourceId;
        ResourceName = resourceName;
        ResourceType = resourceType;
        DeduplicationKey = BuildDeduplicationKey(alertRuleId, resourceId, resourceType, deduplicationComponent);
        OpenIncidentKey = DeduplicationKey;
        CreatedAt = DateTime.UtcNow;
        UpdatedAt = CreatedAt;
    }

    public Result Acknowledge(Guid actorId, DateTime utcNow)
    {
        if (AcknowledgedAt is not null)
            return Result.Failure(new BadRequestError("Alert already acknowledged."));

        AcknowledgedByActorId = actorId;
        AcknowledgedAt = utcNow;
        UpdatedAt = utcNow;

        return Result.Success();
    }

    public Result Resolve(Guid actorId, DateTime utcNow, string? resolutionNote = null)
    {
        if (ResolvedAt is not null)
            return Result.Failure(new BadRequestError("Alert already resolved."));

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

        return Result.Success();
    }

    public static AlertEvent FromPersistence(
        Guid id,
        Guid alertRuleId,
        AlertType type,
        AlertSeverity severity,
        AlertEventInfo info,
        Guid resourceId,
        string resourceName,
        AlertResourceType resourceType,
        string deduplicationKey,
        string? openIncidentKey,
        Guid? acknowledgedByActorId,
        DateTime? acknowledgedAt,
        Guid? resolvedByActorId,
        DateTime? resolvedAt,
        string? resolutionNote,
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
            resourceName: resourceName,
            resourceType: resourceType)
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

        return alertEvent;
    }

    private static string BuildDeduplicationKey(
        Guid alertRuleId,
        Guid resourceId,
        AlertResourceType resourceType,
        string? deduplicationComponent)
    {
        Span<byte> buffer = stackalloc byte[16 + 16 + 4];

        alertRuleId.TryWriteBytes(buffer);
        resourceId.TryWriteBytes(buffer[16..]);
        BinaryPrimitives.WriteInt32LittleEndian(buffer[32..], (int)resourceType);

        if (string.IsNullOrEmpty(deduplicationComponent))
        {
            Span<byte> hash = stackalloc byte[32];
            SHA256.TryHashData(buffer, hash, out _);

            return Convert.ToHexString(hash);
        }

        var componentByteCount = Encoding.UTF8.GetByteCount(deduplicationComponent);
        byte[]? rentedBuffer = null;
        Span<byte> componentBuffer = componentByteCount <= 256
            ? stackalloc byte[componentByteCount]
            : (rentedBuffer = ArrayPool<byte>.Shared.Rent(componentByteCount));

        try
        {
            var written = Encoding.UTF8.GetBytes(deduplicationComponent, componentBuffer);

            using var incrementalHash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            incrementalHash.AppendData(buffer);
            incrementalHash.AppendData(componentBuffer[..written]);

            return Convert.ToHexString(incrementalHash.GetHashAndReset());
        }
        finally
        {
            if (rentedBuffer is not null)
                ArrayPool<byte>.Shared.Return(rentedBuffer);
        }
    }
}
