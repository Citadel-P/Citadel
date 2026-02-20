namespace Infrastructure.Persistence.Dtos;

internal sealed record AlertRuleStateDto(
    Guid AlertRuleId,
    Guid? ResourceId,
    int ConsecutiveMatches,
    DateTime? LastTriggeredAt,
    Guid CreatedByActorId,
    DateTime CreatedAt);
