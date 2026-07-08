using Application.Configs;
using Application.Features.Automation.Models;
using Application.Services;
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
using Microsoft.Extensions.Options;

namespace Application.Features.Automation.Commands;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Write)]
public sealed record UpdateAutomationAction(
    Guid Id,
    UpdateAutomationActionInputModel Action,
    bool UpdateDescription,
    bool UpdateScheduleCron,
    bool UpdateWebhook) : ICommand<Result<AutomationActionResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Action.Description).MaximumLength(600).When(x => x.Action.Description is not null);
            RuleFor(x => x.Action.Code).NotEmpty().MaximumLength(262_144).When(x => x.Action.Code is not null);
            RuleFor(x => x.Action.DefaultArgsJson).MaximumLength(65_536).When(x => x.Action.DefaultArgsJson is not null);
            RuleFor(x => x.Action.ScheduleCron).MaximumLength(128).When(x => x.Action.ScheduleCron is not null);
            RuleFor(x => x.Action.ScheduleTimeZone).MaximumLength(128).When(x => x.Action.ScheduleTimeZone is not null);
            RuleFor(x => x.Action.Webhook!.Secret).MaximumLength(256).When(x => x.Action.Webhook?.Secret is not null);
            RuleFor(x => x.Action.Webhook!.BranchFilter).MaximumLength(256).When(x => x.Action.Webhook?.BranchFilter is not null);
        }
    }
}

internal sealed class UpdateAutomationActionHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IOptions<AutomationOptions> options)
    : ICommandHandler<UpdateAutomationAction, Result<AutomationActionResult>>
{
    private readonly AutomationOptions options = options.Value;

    public async ValueTask<Result<AutomationActionResult>> Handle(UpdateAutomationAction command, CancellationToken cancellationToken)
    {
        var action = await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken);
        if (action is null)
            return Result.Failure<AutomationActionResult>(new NotFoundError("Automation action not found."));

        var input = command.Action;
        if (input.DefaultArgsJson is not null)
        {
            var argsResult = AutomationInputValidation.ValidateJsonObject(input.DefaultArgsJson, "Default args");
            if (argsResult.IsFailure(out var argsError))
                return Result.Failure<AutomationActionResult>(argsError);
        }

        if (input.TimeoutSeconds.HasValue && (input.TimeoutSeconds.Value < 1 || input.TimeoutSeconds.Value > options.MaxTimeoutSeconds))
        {
            return Result.Failure<AutomationActionResult>(
                new BadRequestError($"Timeout must be between 1 and {options.MaxTimeoutSeconds} seconds."));
        }

        var oldAction = AutomationActionActivity.ToSnapshot(action);

        action.Update(
            input.Description,
            command.UpdateDescription,
            input.Code,
            input.DefaultArgsJson,
            input.Enabled,
            input.ScheduleEnabled,
            input.ScheduleCron,
            command.UpdateScheduleCron,
            input.ScheduleTimeZone,
            input.Webhook,
            command.UpdateWebhook,
            input.TimeoutSeconds,
            input.AlertOnFailure,
            input.RunAsActorId);

        try
        {
            action.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<AutomationActionResult>(new BadRequestError(ex.Message));
        }

        await unitOfWork.AutomationActions.UpdateAsync(action, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: action.Name,
                eventType: ActivityEventType.ActionUpdated,
                status: ActivityStatus.Success,
                info: new AutomationActionUpdated(oldAction, AutomationActionActivity.ToSnapshot(action))),
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new AutomationActionResult(action));
    }
}
