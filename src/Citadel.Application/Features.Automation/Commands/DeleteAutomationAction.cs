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

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Write)]
public sealed record DeleteAutomationAction(Guid Id) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeleteAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
        }
    }
}

internal sealed class DeleteAutomationActionHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor)
    : ICommandHandler<DeleteAutomationAction, Result>
{
    public async ValueTask<Result> Handle(DeleteAutomationAction command, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken);
        if (action is null)
            return Result.Failure(new NotFoundError("Automation action not found."));

        await unitOfWork.AutomationActions.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: action.Name,
                eventType: ActivityEventType.ActionDeleted,
                status: ActivityStatus.Success,
                info: new AutomationActionDeleted(AutomationActionActivity.ToSnapshot(action))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
