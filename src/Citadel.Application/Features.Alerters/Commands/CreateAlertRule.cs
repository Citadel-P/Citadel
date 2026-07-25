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
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.Alert, PermissionLevel.Write)]
public sealed record CreateAlertRule(
    string? Name,
    string? Description,
    AlertType Type,
    AlertSeverity Severity,
    int? CooldownSeconds,
    AlertRuleStatus Status,
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
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService) : ICommandHandler<CreateAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(CreateAlertRule command, CancellationToken cancellationToken)
    {
        var entitlement = await entitlementService.EnsureEnabledAsync(
            LicenseCapability.AdvancedAlerting,
            cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<AlertRule>(entitlementError);

        var actorId = userContext.Current.ActorId;
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
            name: command.Name,
            description: command.Description,
            type: command.Type,
            severity: command.Severity,
            cooldownSeconds: command.CooldownSeconds,
            status: command.Status,
            createdByActorId: actorId,
            requiredMatches: command.RequiredMatches,
            threshold: command.Threshold,
            channelIds: command.Channels?.ToList(),
            limitedTo: command.LimitedTo?.ToList(),
            quietHours: command.QuietHours?.ToList());

        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: alertRule.Id,
                        platformId: null,
                        resourceName: alertRule.Name,
                        status: ActivityStatus.Success,
                        eventType: ActivityEventType.AlertRuleCreated,
                        info: new AlertRuleCreated(alertRule.ToSnapshot())
                        );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.AlertRules.AddAlertRuleAsync(alertRule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(alertRule);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return alertRule;
    }
}
