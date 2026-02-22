namespace Infrastructure.Persistence.Dtos;

internal record AlertEventDto(
    Guid Id,
    Guid AlertRuleId,
    string Type,
    string Severity,
    string Info,
    Guid? ResourceId,
    string ResourceType,
    DateTime CreatedAt);
