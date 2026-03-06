using Application.Features.Alerters.Commands;
using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRuleInput(
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
        Type, Severity, CooldownSeconds, Status,
        RequiredMatches, Threshold,
        ChannelIds,
        LimitedTo, QuietHours);
}

public sealed record AlertChannelInput(
    string Name,
    AlertDestination AlertDestination,
    string Url,
    bool IsActive)
{
    internal CreateAlertChannel ToCommand() => new(Name, AlertDestination, Url, IsActive);
}
