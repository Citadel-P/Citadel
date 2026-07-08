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
public sealed record TestAutomationAction(Guid Id, TestAutomationActionInputModel Input) : IStreamCommand<AutomationActionRunStreamItem>
{
    internal sealed class Validator : AbstractValidator<TestAutomationAction>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Input.Code).NotEmpty().MaximumLength(262_144);
            RuleFor(x => x.Input.ArgsJson).MaximumLength(65_536).When(x => x.Input.ArgsJson is not null);
            RuleFor(x => x.Input.DefaultArgsJson).MaximumLength(65_536).When(x => x.Input.DefaultArgsJson is not null);
        }
    }
}

internal sealed class TestAutomationActionHandler(
    IAutomationRunQueueService queueService,
    IAutomationExecutionService executionService,
    IUserContextAccessor userContextAccessor)
    : IStreamCommandHandler<TestAutomationAction, AutomationActionRunStreamItem>
{
    public async IAsyncEnumerable<AutomationActionRunStreamItem> Handle(
        TestAutomationAction command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var result = await queueService.QueueDraftTestAsync(
            command.Id,
            command.Input.Code,
            command.Input.ArgsJson,
            command.Input.DefaultArgsJson,
            command.Input.TimeoutSeconds,
            command.Input.RunAsActorId,
            userContextAccessor.Current.ActorId,
            cancellationToken);

        if (result.IsFailure(out var error, out var run))
        {
            yield return Error(error);
            yield break;
        }

        yield return new AutomationActionRunStreamItem(
            RunId: run.Id,
            Status: ActionRunStatus.Queued.ToString(),
            ProgressMessage: $"Test run for \"{run.ActionName}\" queued.");

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
