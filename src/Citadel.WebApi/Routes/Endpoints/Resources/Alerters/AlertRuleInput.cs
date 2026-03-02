using Application.Features.Alerters.Commands;
using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRuleInput(
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    bool IsEnabled,
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<Guid>? Channels = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null)
{
    internal CreateAlertRule ToCommand() => new(
        Type, Severity, CooldownSeconds, IsEnabled,
        RequiredMatches, Threshold,
        Channels,
        LimitedTo, QuietHours);
}

public sealed record AlertChannelInput(
    AlertDestination AlertDestination,
    string Url,
    bool IsActive)
{
    internal CreateAlertChannel ToCommand() => new(AlertDestination, Url, IsActive);
}
