using Domain.Entities.Alerts;

namespace Domain.Contracts.Resources.Alerts;

public sealed record AlertRuleSnapshot(
    Guid Id,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    int? RequiredMatches,
    double? Threshold,
    AlertRuleStatus Status,
    IEnumerable<Guid> ChannelIds,
    IEnumerable<AlertRuleLimitedTo> LimitedTo,
    IEnumerable<AlertRuleQuietHour> QuietHours);

public static class AlertRuleSnapshotExtensions
{
    public static AlertRuleSnapshot ToSnapshot(this AlertRule alertRule, Guid? id = null)
        => new (
            Id: id ?? alertRule.Id,
            Type: alertRule.Type,
            Severity: alertRule.Severity,
            CooldownSeconds: alertRule.CooldownSeconds,
            RequiredMatches: alertRule.RequiredMatches,
            Threshold: alertRule.Threshold,
            Status: alertRule.Status,
            ChannelIds: alertRule.ChannelIds,
            LimitedTo: alertRule.LimitedTo,
            QuietHours: alertRule.QuietHours);
}
