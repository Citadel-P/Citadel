using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record PatchAlertRule(Guid Id, JsonMergePatchDocument<AlertRule> Patch) : ICommand<Result<AlertRule>>
{
    internal sealed class Validator : AbstractValidator<PatchAlertRule>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Patch).NotNull();
        }
    }
}

internal sealed class PatchAlertRuleHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<PatchAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(PatchAlertRule command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var rule = await unitOfWork.AlertRules.GetByIdAsync(command.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<AlertRule>(new NotFoundError("The provided alert rule does not exist"));
        }

        var oldRule = AlertRule.FromPersistence(
            id: rule.Id,
            type: rule.Type,
            severity: rule.Severity,
            cooldownSeconds: rule.CooldownSeconds,
            isEnabled: rule.IsEnabled,
            createdByActorId: rule.CreatedByActorId,
            createdAt: rule.CreatedAt,
            requiredMatches: rule.RequiredMatches,
            threshold: rule.Threshold,
            channels: rule.Channels,
            limitedTo: rule.LimitedTo,
            quietHours: rule.QuietHours);

        AlertRule patchedRule;
        try
        {
            patchedRule = command.Patch.ApplyTo(rule, AlertRuleJsonContext.Default.AlertRule);
        }
        catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
        {
            return Result.Failure<AlertRule>(new BadRequestError(ex.Message));
        }

        if (patchedRule.Channels.Count > 0)
        {
            foreach (var channelId in patchedRule.Channels)
            {
                var channel = await unitOfWork.AlertRules.GetChannelByIdAsync(channelId, cancellationToken);
                if (channel is null)
                {
                    return Result.Failure<AlertRule>(new NotFoundError($"Alert channel does not exist: {channelId}"));
                }
            }
        }

        rule.PartialUpdate(
            type: patchedRule.Type,
            severity: patchedRule.Severity,
            cooldownSeconds: patchedRule.CooldownSeconds,
            isEnabled: patchedRule.IsEnabled,
            channels: patchedRule.Channels,
            requiredMatches: patchedRule.RequiredMatches,
            threshold: patchedRule.Threshold,
            limitedTo: patchedRule.LimitedTo,
            quietHours: patchedRule.QuietHours);

        var newRule = AlertRule.FromPersistence(
            id: rule.Id,
            type: rule.Type,
            severity: rule.Severity,
            cooldownSeconds: rule.CooldownSeconds,
            isEnabled: rule.IsEnabled,
            createdByActorId: rule.CreatedByActorId,
            createdAt: rule.CreatedAt,
            requiredMatches: rule.RequiredMatches,
            threshold: rule.Threshold,
            channels: rule.Channels,
            limitedTo: rule.LimitedTo,
            quietHours: rule.QuietHours);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: rule.Id,
            platformId: null,
            resourceName: rule.Type.ToString(),
            status: ActivityStatus.Success,
            eventType: ActivityEventType.AlerterUpdated,
            info: new AlerterUpdated(oldRule, newRule)
        );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.AlertRules.UpdateAsync(rule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(rule);

        return rule;
    }
}
