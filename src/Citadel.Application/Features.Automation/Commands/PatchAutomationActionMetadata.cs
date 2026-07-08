using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Automation.Commands;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Write)]
public sealed record PatchAutomationActionMetadata(Guid Id, string? Description) : ICommand<Result<AutomationAction>>
{
    internal sealed class Validator : AbstractValidator<PatchAutomationActionMetadata>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Description).MaximumLength(600).When(x => x.Description is not null);
        }
    }
}

internal sealed class PatchAutomationActionMetadataHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<PatchAutomationActionMetadata, Result<AutomationAction>>
{
    public async ValueTask<Result<AutomationAction>> Handle(PatchAutomationActionMetadata command, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken);
        if (action is null)
            return Result.Failure<AutomationAction>(new NotFoundError("Automation action not found."));

        action.UpdateDescription(command.Description);

        try
        {
            action.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<AutomationAction>(new BadRequestError(ex.Message));
        }

        await unitOfWork.AutomationActions.UpdateAsync(action, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(action);
    }
}
