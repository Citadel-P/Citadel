using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.ErrorTypes;
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
    int? RequiredMatches = null,
    double? Threshold = null,
    IEnumerable<Guid>? Channels = null,
    IEnumerable<AlertRuleLimitedTo>? LimitedTo = null,
    IEnumerable<AlertRuleQuietHour>? QuietHours = null) : ICommand<Result<AlertRule>>
{
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

        }
    }
}

internal sealed class CreateAlertRuleHandler(
    IUnitOfWork unitOfWork, 
    AlertRuleCache alertRuleCache,
    IActivityStreamManager activityHub,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(CreateAlertRule command, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var actorId = user.GetActorId();

        if (command.Channels is not null)
        {
            foreach (var channelId in command.Channels)
            {
                var channel = await unitOfWork.AlertRules.GetChannelByIdAsync(channelId, cancellationToken);
                if (channel is null)
                {
                    return Result.Failure<AlertRule>(new NotFoundError($"Alert channel does not exist: {channelId}"));
                }
            }
        }

        var alertRule = new AlertRule(
            type: command.Type,
            severity: command.Severity,
            cooldownSeconds: command.CooldownSeconds,
            isEnabled: command.IsEnabled,
            createdByActorId: actorId,
            requiredMatches: command.RequiredMatches,
            threshold: command.Threshold,
            channels: command.Channels?.ToList(),
            limitedTo: command.LimitedTo?.ToList(),
            quietHours: command.QuietHours?.ToList());

        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: alertRule.Id,
                        platformId: null,
                        resourceName: alertRule.Type.ToString(),
                        status: ActivityStatus.Success,
                        eventType: ActivityEventType.AlerterCreated,
                        info: new AlerterCreated(alertRule)
                        );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.AlertRules.AddAlertRuleAsync(alertRule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(alertRule);

        return alertRule;
    }
}
