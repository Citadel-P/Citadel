using Domain;
using Domain.Entities.Alerts;

namespace Infrastructure.Persistence.Dtos;

internal record AlertRuleDto(
    Guid Id,
    string Type,
    int? CooldownSeconds,
    string Status,
    string Severity,
    string LimitedTo,
    string QuietHours,
    int? RequiredMatches,
    double? Threshold,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    string ChannelIds
    );

internal record AlertChannelDto(
    Guid Id,
    string Name,
    string AlertDestination,
    string Url,
    bool IsActive,
    Guid CreatedByActorId,
    DateTime CreatedAt
    )
{
    internal AlertChannel ToDomain() => AlertChannel.FromPersistence(
        Id,
        Name,
        Enum.Parse<AlertDestination>(AlertDestination),
        Url,
        IsActive,
        CreatedByActorId);
}

internal record AlertRuleChannelLinkDto(Guid AlertRuleId, Guid AlertChannelId);