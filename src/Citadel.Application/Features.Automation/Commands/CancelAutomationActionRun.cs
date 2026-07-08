using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Automation.Commands;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Execute)]
public sealed record CancelAutomationActionRun(Guid Id, Guid RunId) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<CancelAutomationActionRun>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.RunId).NotEmpty();
        }
    }
}

internal sealed class CancelAutomationActionRunHandler(
    IUnitOfWork unitOfWork,
    IAutomationRunCoordinator runCoordinator,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<CancelAutomationActionRun, Result>
{
    public async ValueTask<Result> Handle(CancelAutomationActionRun command, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken);
        if (action is null)
            return Result.Failure(new NotFoundError("Automation action not found."));

        var run = await unitOfWork.ActionRuns.GetAsync(command.RunId, cancellationToken);
        if (run is null || run.ActionId != action.Id)
            return Result.Failure(new NotFoundError("Automation action run not found."));

        runCoordinator.Cancel(run.Id);
        var rows = await unitOfWork.ActionRuns.CancelQueuedOrRunningAsync(run.Id, DateTime.UtcNow, "Run cancelled.", cancellationToken);
        if (rows == 0)
            return Result.Failure(new ConflictError("Only queued or running action runs can be cancelled."));

        await unitOfWork.AutomationActions.MarkIdleAsync(action.Id, run.Id, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: action.Name,
                eventType: ActivityEventType.ActionRunCancelled,
                status: ActivityStatus.Warning,
                info: new AutomationActionRunCancelled(run.Id, run.Trigger)),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
