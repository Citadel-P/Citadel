using Application.Features.Deployments.Notifications;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Alerts;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.Alert, PermissionLevel.Execute)]
public sealed record DeleteAlertRules(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteAlertRules>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class DeleteAlertRulesHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILogger<DeleteAlertRulesHandler> logger) : ICommandHandler<DeleteAlertRules, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertRules command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var requestedIds = command.Ids.Distinct().ToArray();
        var rulesToDelete = (await unitOfWork.AlertRules.GetAllAsync(requestedIds, cancellationToken) ?? [])
            .ToArray();

        if (requestedIds.Length == 0 || rulesToDelete.Length != requestedIds.Length)
        {
            return Result.Failure(new NotFoundError("One or more alert rules were not found."));
        }

        var ids = rulesToDelete.Select(s => s.Id).ToArray();
        var notifications = new List<INotificationWorkItem>(ids.Length);
        foreach (var rule in rulesToDelete)
        {
            var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: rule.Id,
                platformId: null,
                resourceName: rule.Name,
                status: ActivityStatus.Success,
                eventType: ActivityEventType.AlertRuleDeleted,
                info: new AlertRuleDeleted(rule.ToSnapshot())
            );

            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            notifications.Add(new ActivityNotificationWorkItem(
                activityHub,
                await activity.AssignActor(unitOfWork, cancellationToken)));
        }

        var result = await unitOfWork.AlertRules.RemoveRangeAsync(ids, cancellationToken);
        if (result != ids.Length)
        {
            await unitOfWork.RollbackAsync();
            return Result.Failure(new ConflictError(
                "The alert-rule set changed while deletion was in progress."));
        }

        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Remove(ids);
        using var notificationCancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        foreach (var notification in notifications)
        {
            try
            {
                await notificationQueue.EnqueueAsync(notification, notificationCancellation.Token);
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Failed to publish deleted alert-rule activity");
            }
        }

        return Result.Success();
    }
}
