using Application.Features.Alerters.Commands;
using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record CreateAlertRuleInput(
    string? Name,
    string? Description,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    AlertRuleStatus Status,
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<Guid>? ChannelIds = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null)
{
    internal CreateAlertRule ToCommand() => new(
        Name, Description, Type, Severity, CooldownSeconds, Status,
        RequiredMatches, Threshold,
        ChannelIds,
        LimitedTo, QuietHours);
}
