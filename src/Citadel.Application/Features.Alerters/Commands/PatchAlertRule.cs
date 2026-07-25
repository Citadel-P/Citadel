using Application.Features.Deployments.Notifications;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Alerts;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.Alert, PermissionLevel.Write)]
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
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService) : ICommandHandler<PatchAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(PatchAlertRule command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var rule = await unitOfWork.AlertRules.GetByIdAsync(command.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<AlertRule>(new NotFoundError("The provided alert rule does not exist"));
        }

        var oldRule = AlertRule.FromPersistence(
            id: rule.Id,
            name: rule.Name,
            description: rule.Description,
            type: rule.Type,
            severity: rule.Severity,
            cooldownSeconds: rule.CooldownSeconds,
            status: rule.Status,
            createdByActorId: rule.CreatedByActorId,
            createdAt: rule.CreatedAt,
            requiredMatches: rule.RequiredMatches,
            threshold: rule.Threshold,
            channelIds: rule.ChannelIds,
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

        var advancedConfigurationChanged =
            rule.Type != patchedRule.Type
            || rule.Severity != patchedRule.Severity
            || rule.CooldownSeconds != patchedRule.CooldownSeconds
            || rule.RequiredMatches != patchedRule.RequiredMatches
            || rule.Threshold != patchedRule.Threshold
            || !rule.LimitedTo.SequenceEqual(patchedRule.LimitedTo)
            || !rule.QuietHours.SequenceEqual(patchedRule.QuietHours);
        var addedChannel = patchedRule.ChannelIds.Except(rule.ChannelIds).Any();
        var enabledCustomRule =
            rule.CreatedByActorId != Constants.SystemId
            && rule.Status != patchedRule.Status
            && patchedRule.Status == AlertRuleStatus.Enabled;

        if (advancedConfigurationChanged
            || (rule.CreatedByActorId != Constants.SystemId && addedChannel)
            || enabledCustomRule)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AdvancedAlerting,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<AlertRule>(entitlementError);
        }

        if (patchedRule.ChannelIds.Count > 0)
        {
            foreach (var channelId in patchedRule.ChannelIds)
            {
                var channel = await unitOfWork.AlertRules.GetChannelByIdAsync(channelId, cancellationToken);
                if (channel is null)
                {
                    return Result.Failure<AlertRule>(new NotFoundError($"Alert channel does not exist: {channelId}"));
                }
            }
        }

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: rule.Id,
            platformId: null,
            resourceName: rule.Name,
            status: ActivityStatus.Success,
            eventType: ActivityEventType.AlertRuleUpdated,
            info: new AlertRuleUpdated(oldRule.ToSnapshot(command.Id), patchedRule.ToSnapshot(command.Id))
        );

        rule.PartialUpdate(
            name: rule.Name,
            description: rule.Description,
            type: patchedRule.Type,
            severity: patchedRule.Severity,
            cooldownSeconds: patchedRule.CooldownSeconds,
            status: patchedRule.Status,
            channelIds: patchedRule.ChannelIds,
            requiredMatches: patchedRule.RequiredMatches,
            threshold: patchedRule.Threshold,
            limitedTo: patchedRule.LimitedTo,
            quietHours: patchedRule.QuietHours);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.AlertRules.UpdateAsync(rule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(rule);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return rule;
    }
}
