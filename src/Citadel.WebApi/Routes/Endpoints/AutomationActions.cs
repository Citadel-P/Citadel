using Application.Features.Automation.Commands;
using Application.Features.Automation.Queries;
using Application.Models;
using Application.Permissions;
using Domain.Contracts.Resources.Automation;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Automation;

namespace WebApi.Routes.Endpoints;

public static class AutomationActions
{
    public static async Task<Results<Ok<AutomationActionsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetAutomationActions(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionsView.Map);
    }

    public static async Task<Results<Ok<AutomationActionView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Automation action ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAutomationAction(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionView.Map);
    }

    public static async Task<Results<Ok<AutomationActionView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] AutomationActionInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateAutomationAction(input.ToModel()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionView.Map);
    }

    public static async Task<Results<Ok<AutomationActionView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Automation action ID")] Guid id,
        UpdateAutomationActionInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateAutomationActionInput(),
            ApplicationJsonContext.Default.UpdateAutomationActionInput);

        var result = await mediator.Send(
            new UpdateAutomationAction(
                id,
                input.ToModel(),
                patchInput.ContainsProperty("description"),
                patchInput.ContainsProperty("scheduleCron"),
                patchInput.ContainsProperty("webhook")),
            cancellationToken);

        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionView.Map);
    }

    public static async Task<Results<Ok<AutomationActionView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameAutomationAction(renameResource.Id, renameResource.Name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionView.Map);
    }

    public static async Task<Results<Ok<AutomationActionView>, ProblemHttpResult>> UpdateMetadata(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Automation action ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new PatchResourceMetadata(string.Empty, []),
            ApplicationJsonContext.Default.PatchResourceMetadata);

        var result = await mediator.Send(new PatchAutomationActionMetadata(id, input.Description), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, AutomationActionView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteAutomationAction(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async IAsyncEnumerable<AutomationActionRunStreamItem> Run(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromBody] RunAutomationActionInput input,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new RunAutomationAction(id, input.ToModel()), cancellationToken))
            yield return item;
    }

    public static async IAsyncEnumerable<AutomationActionRunStreamItem> Test(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromBody] TestAutomationActionInput input,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new TestAutomationAction(id, input.ToModel()), cancellationToken))
            yield return item;
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Cancel(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromRoute][Description("Automation action run ID")] Guid runId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CancelAutomationActionRun(id, runId), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<AutomationActionRunsView>, ProblemHttpResult>> ListRuns(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromQuery] int? limit,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAutomationActionRuns(id, limit ?? 50), cancellationToken);
        return EndpointHandlers.HandleResult(result, AutomationActionRunsView.Map);
    }

    public static async Task<Results<Ok<AutomationActionRunView>, ProblemHttpResult>> GetRun(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromRoute][Description("Automation action run ID")] Guid runId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAutomationActionRun(id, runId), cancellationToken);
        return EndpointHandlers.HandleResult(result, AutomationActionRunView.Map);
    }

    public static async Task<Results<Ok<AutomationActionRunLogsView>, ProblemHttpResult>> GetRunLogs(
        IMediator mediator,
        [FromRoute][Description("Automation action ID")] Guid id,
        [FromRoute][Description("Automation action run ID")] Guid runId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAutomationActionRun(id, runId), cancellationToken);
        return EndpointHandlers.HandleResult(result, AutomationActionRunLogsView.Map);
    }

}
