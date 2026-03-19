namespace Infrastructure.Persistence.Dtos;

internal record AlertEventDto(
    Guid Id,
    Guid AlertRuleId,
    string Type,
    string Severity,
    string Info,
    Guid? ResourceId,
    string ResourceName,
    string ResourceType,
    string DeduplicationKey,
    string? OpenIncidentKey,
    Guid? AcknowledgedByActorId,
    DateTime? AcknowledgedAt,
    Guid? ResolvedByActorId,
    DateTime? ResolvedAt,
    string? ResolutionNote,
    DateTime CreatedAt,
    DateTime UpdatedAt);
