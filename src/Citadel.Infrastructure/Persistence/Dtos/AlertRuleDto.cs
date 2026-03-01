using Domain;
using Domain.Entities.Alerts;

namespace Infrastructure.Persistence.Dtos;

internal record AlertRuleDto(
    Guid Id,
    string Type,
    int? CooldownSeconds,
    bool IsEnabled,
    string Severity,
    string LimitedTo,
    string QuietHours,
    int? RequiredMatches,
    double? Threshold,
    Guid CreatedByActorId,
    DateTime CreatedAt
    )
{
    public ICollection<AlertChannelDto> Channels { get; init; } = [];
}

internal record AlertChannelDto(
    Guid Id,
    string AlertDestination,
    string Url,
    bool IsActive,
    Guid CreatedByActorId,
    DateTime CreatedAt
    )
{
    internal AlertChannel ToDomain() => AlertChannel.FromPersistence(
        Id,
        Enum.Parse<AlertDestination>(AlertDestination),
        Url,
        IsActive,
        CreatedByActorId);
}

internal record AlertRuleChannelLinkDto(Guid AlertRuleId, Guid AlertChannelId);