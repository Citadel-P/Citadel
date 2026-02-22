namespace Infrastructure.Persistence.Dtos;

internal record AlertRuleDto(
    Guid Id,
    string Type,
    int CooldownSeconds,
    bool IsEnabled,
    string Scope,
    string Severity,
    string LimitedTo,
    string QuietHours,
    int? RequiredMatches,
    double? Threshold,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    Guid? State_ResourceId = null,
    int? State_ConsecutiveMatches = null,
    DateTime? State_LastTriggeredAt = null,
    Guid? State_CreatedByActorId = null,
    DateTime? State_CreatedAt = null
    )
{
    public ICollection<AlertChannelDto> Channels { get; init; } = [];
}

internal record AlertChannelDto(
    Guid Id,
    Guid AlertRuleId,
    string AlertDestination,
    string Url,
    bool IsActive,
    Guid CreatedByActorId,
    DateTime CreatedAt
    );