using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record CreateAlertRule(
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    bool IsEnabled,
    AlertScope Scope,
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<CreateAlertRule.ChannelInput>? Channels = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null) : ICommand<Result<AlertRule>>
{
    public sealed record ChannelInput(
        AlertDestination AlertDestination,
        string Url,
        bool IsActive);

    internal sealed class Validator : AbstractValidator<CreateAlertRule>
    {
        public Validator()
        {
            RuleFor(x => x.CooldownSeconds)
                .InclusiveBetween(10, 86400)
                .WithMessage("Cooldown must be between 10s and 24h.");

            When(x => AlertTypeMetadata.IsThreshold(x.Type), () =>
            {
                RuleFor(x => x.RequiredMatches).NotNull();
                RuleFor(x => x.Threshold).NotNull();
            });

            When(x => !AlertTypeMetadata.IsThreshold(x.Type), () =>
            {
                RuleFor(x => x.RequiredMatches).Null().WithMessage("Non-threshold alerts must not define RequiredMatches.");
                RuleFor(x => x.Threshold).Null().WithMessage("Non-threshold alerts must not define Threshold.");
            });

            When(x => x.Scope == AlertScope.Specific, () =>
            {
                RuleFor(x => x.LimitedTo).NotNull().NotEmpty()
                    .WithMessage("LimitedTo must contain at least one resource when Scope is Specific.");
            });

            When(x => x.Scope == AlertScope.All, () =>
            {
                RuleFor(x => x.LimitedTo).Must(l => l is null || !l.Any())
                    .WithMessage("LimitedTo must be empty when Scope is All.");
            });
        }
    }
}

internal sealed class CreateAlertRuleHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(CreateAlertRule command, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var actorId = user.GetActorId();
        var channels = command.Channels?.Select(c => new AlertChannel(c.AlertDestination, c.Url, c.IsActive, actorId)).ToList();

        var alertRule = new AlertRule(
            type: command.Type,
            severity: command.Severity,
            cooldownSeconds: command.CooldownSeconds,
            isEnabled: command.IsEnabled,
            scope: command.Scope,
            createdByActorId: actorId,
            requiredMatches: command.RequiredMatches,
            threshold: command.Threshold,
            channels: channels,
            limitedTo: command.LimitedTo?.ToList(),
            quietHours: command.QuietHours?.ToList());

        await unitOfWork.AlertRules.AddAlertRuleAsync(alertRule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return alertRule;
    }
}
