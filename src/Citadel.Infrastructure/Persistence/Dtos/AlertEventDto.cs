namespace Infrastructure.Persistence.Dtos;

internal record AlertEventDto(
    Guid Id,
    Guid AlertRuleId,
    Guid CreatedByActorId,
    string Type,
    string Severity,
    string Info,
    Guid? ResourceId,
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
