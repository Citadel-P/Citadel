namespace Infrastructure.Persistence.Dtos;

internal record AlertRuleDto(
    Guid Id,
    string Url,
    string Type,
    int CooldownSeconds,
    bool IsEnabled,
    string Scope,
    string Severity,
    string LimitedTo,
    string QuietHours,
    int RequiredMatches,
    double Threshold,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    Guid? State_ResourceId = null,
    int? State_ConsecutiveMatches = null,
    DateTime? State_LastTriggeredAt = null,
    Guid? State_CreatedByActorId = null,
    DateTime? State_CreatedAt = null
    );