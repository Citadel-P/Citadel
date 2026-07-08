using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Automation;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Automation.Commands;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Write)]
public sealed record RenameAutomationAction(Guid Id, string Name) : ICommand<Result<AutomationAction>>
{
    internal sealed class Validator : AbstractValidator<RenameAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
        }
    }
}

internal sealed class RenameAutomationActionHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor)
    : ICommandHandler<RenameAutomationAction, Result<AutomationAction>>
{
    public async ValueTask<Result<AutomationAction>> Handle(RenameAutomationAction command, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken);
        if (action is null)
            return Result.Failure<AutomationAction>(new NotFoundError("Automation action not found."));

        if (await unitOfWork.AutomationActions.ExistsByNameExceptAsync(command.Name, command.Id, cancellationToken))
            return Result.Failure<AutomationAction>(new ConflictError("Automation action name already exists."));

        var oldName = action.Name;
        action.Rename(command.Name);

        try
        {
            action.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<AutomationAction>(new BadRequestError(ex.Message));
        }

        await unitOfWork.AutomationActions.UpdateAsync(action, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: action.Name,
                eventType: ActivityEventType.ActionRenamed,
                status: ActivityStatus.Success,
                info: new AutomationActionRenamed(oldName, action.Name)),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(action);
    }
}
