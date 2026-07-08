using Application.Features.Automation.Models;
using Application.Services;
using Domain;
using Domain.Contracts.Resources.Automation;
using Domain.Entities.Automation;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Automation.Commands;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Execute)]
public sealed record RunAutomationAction(Guid Id, RunAutomationActionInputModel Input) : IStreamCommand<AutomationActionRunStreamItem>
{
    internal sealed class Validator : AbstractValidator<RunAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Input.ArgsJson).MaximumLength(65_536).When(x => x.Input.ArgsJson is not null);
        }
    }
}

internal sealed class RunAutomationActionHandler(
    IAutomationRunQueueService queueService,
    IAutomationExecutionService executionService,
    IUserContextAccessor userContextAccessor)
    : IStreamCommandHandler<RunAutomationAction, AutomationActionRunStreamItem>
{
    public async IAsyncEnumerable<AutomationActionRunStreamItem> Handle(
        RunAutomationAction command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var result = await queueService.QueueAsync(
            command.Id,
            ActionRunTrigger.Manual,
            command.Input.ArgsJson,
            command.Input.TimeoutSeconds,
            userContextAccessor.Current.ActorId,
            requireEnabled: true,
            cancellationToken);

        if (result.IsFailure(out var error, out var run))
        {
            yield return Error(error);
            yield break;
        }

        yield return new AutomationActionRunStreamItem(
            RunId: run.Id,
            Status: ActionRunStatus.Queued.ToString(),
            ProgressMessage: $"Action \"{run.ActionName}\" queued.");

        await foreach (var item in executionService.ExecuteQueuedAsync(run.Id, cancellationToken))
            yield return item;
    }

    private static AutomationActionRunStreamItem Error(IError error)
        => new(ErrorMessage: error.Message, Error: new AutomationActionRunStreamError(GetCode(error), error.Message));

    private static int GetCode(IError error)
        => error switch
        {
            NotFoundError => 404,
            ConflictError => 409,
            BadRequestError => 400,
            ForbiddenError => 403,
            UnauthorizedError => 401,
            _ => 500
        };
}
