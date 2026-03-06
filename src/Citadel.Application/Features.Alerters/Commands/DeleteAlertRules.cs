using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Alerts;
using Domain.Entities.Activities;
using Domain.Entities.Alerts;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record DeleteAlertRules(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteAlertRulesHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<DeleteAlertRules, Result>
{
    public async ValueTask<Result> Handle(DeleteAlertRules command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var rulesToDelete = new List<AlertRule>();
        var ids = command.Ids.Distinct().ToArray();
        foreach (var id in ids)
        {
            var rule = await unitOfWork.AlertRules.GetByIdAsync(id, cancellationToken);
            if (rule is not null)
            {
                rulesToDelete.Add(rule);
            }
        }

        if (rulesToDelete.Count == 0)
        {
            return Result.Failure(new NotFoundError("No alert rules found matching the provided IDs for deletion."));
        }

        foreach (var rule in rulesToDelete)
        {
            var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: rule.Id,
                platformId: null,
                resourceName: rule.Type.ToString(),
                status: ActivityStatus.Success,
                eventType: ActivityEventType.AlertRuleDeleted,
                info: new AlertRuleDeleted(rule.ToSnapshot())
            );

            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
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
