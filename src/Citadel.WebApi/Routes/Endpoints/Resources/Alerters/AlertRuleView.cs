using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRuleView(
    Guid Id,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    int? RequiredMatches,
    double? Threshold,
    bool IsEnabled,
    AlertScope Scope,
    IEnumerable<AlertChannelView> Channels,
    IEnumerable<AlertRuleLimitedTo> LimitedTo,
    IEnumerable<AlertRuleQuietHour> QuietHours,
    Guid CreatedByActorId,
    DateTime CreatedAt)
{
    internal static AlertRuleView Map(AlertRule rule)
    {
        return new(
            rule.Id,
            rule.Type,
            rule.Severity,
            rule.CooldownSeconds,
            rule.RequiredMatches,
            rule.Threshold,
            rule.IsEnabled,
            rule.Scope,
            rule.Channels.Select(AlertChannelView.Map),
            rule.LimitedTo,
            rule.QuietHours,
            rule.CreatedByActorId,
            rule.CreatedAt);
    }
}
