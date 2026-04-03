using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record PatchAlertRuleInput(
    string? Description,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    AlertRuleStatus Status,
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<Guid>? ChannelIds = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null);
