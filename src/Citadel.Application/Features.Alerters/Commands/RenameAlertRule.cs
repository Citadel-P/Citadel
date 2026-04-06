using Application.Features.Deployments.Notifications;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.Alert, ResourceAction.Update)]
public sealed record RenameAlertRule(Guid Id, string Name) : ICommand<Result<AlertRule>>
{
    internal sealed class Validator : AbstractValidator<RenameAlertRule>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().MaximumLength(120);
        }
    }
}

internal sealed class RenameAlertRuleHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<RenameAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(RenameAlertRule command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var rule = await unitOfWork.AlertRules.GetByIdAsync(command.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<AlertRule>(new NotFoundError("The provided alert rule does not exist"));
        }

        var oldName = rule.Name;
        rule.PartialUpdate(name: command.Name);

        var activity = new ActivityEvent(
           actorId: actorId,
           resourceId: rule.Id,
           platformId: null,
           resourceName: command.Name,
           status: ActivityStatus.Success,
           eventType: ActivityEventType.AlertRuleRenamed,
           info: new AlertRuleRenamed(oldName, command.Name)
       );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.AlertRules.UpdateAsync(rule, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        alertRuleCache.Upsert(rule);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return rule;
    }
}
