using Application.Features.Automation.Models;
using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Automation.Queries;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Read)]
public sealed record GetAutomationActions : IQuery<Result<AutomationActionListResult>>;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Read)]
public sealed record GetAutomationAction(Guid Id) : IQuery<Result<AutomationAction>>;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Read)]
public sealed record GetAutomationActionRuns(Guid ActionId, int Limit = 50) : IQuery<Result<ActionRunListResult>>;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Read)]
public sealed record GetAutomationActionRun(Guid ActionId, Guid RunId) : IQuery<Result<ActionRun>>;

internal sealed class GetAutomationActionsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetAutomationActions, Result<AutomationActionListResult>>
{
    public async ValueTask<Result<AutomationActionListResult>> Handle(GetAutomationActions query, CancellationToken cancellationToken)
    {
        var actions = (await unitOfWork.AutomationActions.GetAllAsync(cancellationToken)).ToArray();
        var latest = new Dictionary<Guid, ActionRun>();

        foreach (var action in actions)
        {
            var run = await unitOfWork.ActionRuns.GetLatestByActionAsync(action.Id, cancellationToken);
            if (run is not null)
                latest[action.Id] = run;
        }

        return Result.Success(new AutomationActionListResult(actions, latest));
    }
}

internal sealed class GetAutomationActionHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetAutomationAction, Result<AutomationAction>>
{
    public async ValueTask<Result<AutomationAction>> Handle(GetAutomationAction query, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(query.Id, cancellationToken);
        return action is null
            ? Result.Failure<AutomationAction>(new NotFoundError("Automation action not found."))
            : Result.Success(action);
    }
}

internal sealed class GetAutomationActionRunsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetAutomationActionRuns, Result<ActionRunListResult>>
{
    public async ValueTask<Result<ActionRunListResult>> Handle(GetAutomationActionRuns query, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(query.ActionId, cancellationToken);
        if (action is null)
            return Result.Failure<ActionRunListResult>(new NotFoundError("Automation action not found."));

        var runs = await unitOfWork.ActionRuns.GetByActionAsync(query.ActionId, Math.Clamp(query.Limit, 1, 200), cancellationToken);
        return Result.Success(new ActionRunListResult([.. runs]));
    }
}

internal sealed class GetAutomationActionRunHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetAutomationActionRun, Result<ActionRun>>
{
    public async ValueTask<Result<ActionRun>> Handle(GetAutomationActionRun query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.ActionRuns.GetAsync(query.RunId, cancellationToken);
        return run is null || run.ActionId != query.ActionId
            ? Result.Failure<ActionRun>(new NotFoundError("Automation action run not found."))
            : Result.Success(run);
    }
}
