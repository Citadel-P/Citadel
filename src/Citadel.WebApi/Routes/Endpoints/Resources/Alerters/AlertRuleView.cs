using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed partial record AlertRuleView(
    Guid Id,
    string Name,
    string? Description,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    int? RequiredMatches,
    double? Threshold,
    AlertRuleStatus Status,
    IEnumerable<AlertChannelView> Channels,
    IEnumerable<AlertRuleLimitedTo> LimitedTo,
    IEnumerable<AlertRuleQuietHour> QuietHours)
{
    internal static AlertRuleView Map(AlertRule rule)
        => Map(rule, new Dictionary<Guid, AlertChannel>());

    internal static AlertRuleView Map(AlertRule rule, IReadOnlyDictionary<Guid, AlertChannel> channelsById)
    {
        var channels = rule.ChannelIds
            .Select(id => channelsById.GetValueOrDefault(id))
            .Where(c => c is not null)
            .Select(c => AlertChannelView.Map(c!));

        return new(
            rule.Id,
            rule.Name,
            rule.Description,
            rule.Type,
            rule.Severity,
            rule.CooldownSeconds,
            rule.RequiredMatches,
            rule.Threshold,
            rule.Status,
            channels,
            rule.LimitedTo,
            rule.QuietHours);
    }

}


public sealed record AlertRuleConfigView(
    Guid Id,
    string Name,
    string? Description,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    int? RequiredMatches,
    double? Threshold,
    AlertRuleStatus Status,
    IEnumerable<Guid> ChannelIds,
    IEnumerable<AlertRuleLimitedTo> LimitedTo,
    IEnumerable<AlertRuleQuietHour> QuietHours)
{
    internal static AlertRuleConfigView Map(AlertRule rule)
        => new(
            rule.Id,
            rule.Name,
            rule.Description,
            rule.Type,
            rule.Severity,
            rule.CooldownSeconds,
            rule.RequiredMatches,
            rule.Threshold,
            rule.Status,
            rule.ChannelIds,
            rule.LimitedTo,
            rule.QuietHours);
}