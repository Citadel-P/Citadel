using Application.Features.Alerters.Commands;
using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRuleInput(
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    bool IsEnabled,
    AlertScope Scope,
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<AlertChannelInput>? Channels = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null)
{
    internal CreateAlertRule ToCommand() => new(
        Type, Severity, CooldownSeconds, IsEnabled, Scope,
        RequiredMatches, Threshold,
        Channels?.Select(c => new CreateAlertRule.ChannelInput(c.AlertDestination, c.Url, c.IsActive)),
        LimitedTo, QuietHours);
}

public sealed record AlertChannelInput(
    AlertDestination AlertDestination,
    string Url,
    bool IsActive);
