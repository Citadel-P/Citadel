using Application.Features.Deployments.Notifications;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Alerts;
using Domain.Entities.Activities;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
using Hosting.Common;

namespace Application.Features.Alerters.Commands;

public sealed record DeleteAlertRules(IEnumerable<Guid> Ids) : ICommand<Result>;

[RequirePermission(ResourceType.Alert, PermissionLevel.Execute)]
internal sealed class DeleteAlertRulesHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<DeleteAlertRules, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertRules command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var rulesToDelete = await unitOfWork.AlertRules.GetAllAsync(command.Ids, cancellationToken);

        if (rulesToDelete == null || rulesToDelete.Any() == false)
        {
            return Result.Failure(new NotFoundError("No alert rules found matching the provided IDs for deletion."));
        }

        var ids = rulesToDelete.Select(s => s.Id).ToList();
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
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        }

        var result = await unitOfWork.AlertRules.RemoveRangeAsync(ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        if (result > 0)
        {
            alertRuleCache.Remove(ids);
        }

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No alert rules found matching the provided IDs for deletion."));
    }
}
