using Application.Configs;
using Application.Features.Automation.Models;
using Application.Services;
using Application.Services.Licensing;
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
public sealed record CreateAutomationAction(AutomationActionInputModel Action) : ICommand<Result<AutomationActionResult>>
{
    internal sealed class Validator : AbstractValidator<CreateAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Action.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Action.Description).MaximumLength(600).When(x => x.Action.Description is not null);
            RuleFor(x => x.Action.Code).NotEmpty().MaximumLength(262_144);
            RuleFor(x => x.Action.DefaultArgsJson).MaximumLength(65_536).When(x => x.Action.DefaultArgsJson is not null);
            RuleFor(x => x.Action.ScheduleCron).MaximumLength(128).When(x => x.Action.ScheduleCron is not null);
            RuleFor(x => x.Action.ScheduleTimeZone).MaximumLength(128).When(x => x.Action.ScheduleTimeZone is not null);
            RuleFor(x => x.Action.Webhook!.Secret).MaximumLength(256).When(x => x.Action.Webhook?.Secret is not null);
            RuleFor(x => x.Action.Webhook!.BranchFilter).MaximumLength(256).When(x => x.Action.Webhook?.BranchFilter is not null);
        }
    }
}

internal sealed class CreateAutomationActionHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    ILicenseEntitlementService licenseEntitlementService,
    IOptions<AutomationOptions> options)
    : ICommandHandler<CreateAutomationAction, Result<AutomationActionResult>>
{
    private readonly AutomationOptions options = options.Value;

    public async ValueTask<Result<AutomationActionResult>> Handle(CreateAutomationAction command, CancellationToken cancellationToken)
    {
        var input = command.Action;
        if (await unitOfWork.AutomationActions.ExistsByNameAsync(input.Name, cancellationToken))
            return Result.Failure<AutomationActionResult>(new ConflictError("Automation action name already exists."));

        if (input.ScheduleEnabled || input.Webhook?.Enabled == true)
        {
            var entitlement = await licenseEntitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure())
                return Result.Failure<AutomationActionResult>(entitlement.Errors);
        }

        var argsJson = AutomationInputValidation.NormalizeJsonObject(input.DefaultArgsJson);
        var argsResult = AutomationInputValidation.ValidateJsonObject(argsJson, "Default args");
        if (argsResult.IsFailure(out var argsError))
            return Result.Failure<AutomationActionResult>(argsError);

        var timeoutSeconds = input.TimeoutSeconds ?? options.DefaultTimeoutSeconds;
        if (timeoutSeconds < 1 || timeoutSeconds > options.MaxTimeoutSeconds)
        {
            return Result.Failure<AutomationActionResult>(
                new BadRequestError($"Timeout must be between 1 and {options.MaxTimeoutSeconds} seconds."));
        }

        var action = new AutomationAction(
            input.Name,
            input.Description,
            input.Code,
            argsJson,
            input.Enabled,
            input.ScheduleEnabled,
            input.ScheduleCron,
            input.ScheduleTimeZone ?? "UTC",
            input.Webhook,
            timeoutSeconds,
            input.AlertOnFailure,
            input.RunAsActorId.GetValueOrDefault(userContextAccessor.Current.ActorId),
            userContextAccessor.Current.ActorId);

        try
        {
            action.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<AutomationActionResult>(new BadRequestError(ex.Message));
        }

        var affectedRows = await unitOfWork.AutomationActions.AddAsync(
            action,
            cancellationToken,
            input.TagIds,
            userContextAccessor.Current.ActorId);

        if (affectedRows == 0)
            return Result.Failure<AutomationActionResult>(new BadRequestError("One or more tags do not exist."));

        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: action.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: action.Name,
                eventType: ActivityEventType.ActionCreated,
                status: ActivityStatus.Success,
                info: new AutomationActionCreated(AutomationActionActivity.ToSnapshot(action))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new AutomationActionResult(action));
    }
}

internal static class AutomationActionActivity
{
    internal static AutomationActionSnapshot ToSnapshot(AutomationAction action)
        => new(
            action.Id,
            action.Name,
            action.Description,
            action.Code,
            action.DefaultArgsJson,
            action.Enabled,
            action.ScheduleEnabled,
            action.ScheduleCron,
            action.ScheduleTimeZone,
            action.Webhook is null ? null : action.Webhook with { Secret = action.Webhook.Secret.MaskValue() },
            action.TimeoutSeconds,
            action.AlertOnFailure,
            action.RunAsActorId);
}
