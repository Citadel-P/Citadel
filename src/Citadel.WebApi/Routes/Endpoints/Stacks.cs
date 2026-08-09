using Application.Features.Stacks.Commands;
using Application.Features.Stacks.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Stacks;

namespace WebApi.Routes.Endpoints;

public static class Stacks
{
    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStack(stackId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, StackView.Map);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> CheckUpdates(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [Description("The stack id")] Guid stackId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CheckStackUpdates(stackId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, StackView.Map);
    }

    public static async Task<Results<Ok<StackConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStack(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackConfigView.Map);
    }

    public static async Task<Results<Ok<StackBackupSourcePreviewView>, ProblemHttpResult>> GetBackupSourcePreview(
        IMediator mediator,
        [Description("The stack id")] Guid stackId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStackBackupSourcePreview(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackBackupSourcePreviewView.Map);
    }

    public static async Task<Results<Ok<StackDuplicateDraftView>, ProblemHttpResult>> GetDuplicateDraft(IMediator mediator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStackDuplicateDraft(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, draft => StackDuplicateDraftView.Map(draft, stackId));
    }

    public static async Task<Results<Ok<ComposeProjectImportDraftView>, ProblemHttpResult>> GetComposeImportDraft(
        IMediator mediator,
        [Description("Platform id")] Guid platformId,
        [Description("Docker Compose project name")] string projectName,
        [FromQuery] StackImportKind? importKind,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new GetComposeProjectImportDraft(platformId, projectName, importKind),
            cancellationToken);
        return EndpointHandlers.HandleResult(result, ComposeProjectImportDraftView.Map);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> ImportComposeProject(
        IMediator mediator,
        [Description("Platform id")] Guid platformId,
        [Description("Docker Compose project name")] string projectName,
        [FromBody] ImportComposeProjectInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            input.ToCommand(platformId, projectName),
            cancellationToken);
        return EndpointHandlers.HandleResult(result, StackView.Map);
    }

    public static async Task<Results<Ok<ComposeProjectImportValidation>, ProblemHttpResult>> ValidateComposeImportDraft(
        IMediator mediator,
        [Description("Platform id")] Guid platformId,
        [Description("Docker Compose project name")] string projectName,
        [FromBody] ValidateComposeProjectImportInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            input.ToQuery(platformId, projectName),
            cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }

    public static async Task<Results<Ok<StacksView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        [FromQuery] Guid? platformId = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetAllStacks(tags, platformId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, StacksView.Map);
    }

    public static async Task<Results<Ok<StackReleasesView>, ProblemHttpResult>> ListReleases(IMediator mediator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStackReleases(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackReleasesView.Map);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateStackInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackView.Map);
    }

    public static async Task<Results<Ok<SwarmStackCompatibilityReport>, ProblemHttpResult>> PreflightSwarm(
        IMediator mediator,
        [FromBody] SwarmStackPreflightInput request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteStacks(stackIds), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Stack ID")] Guid id,
        StackInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<StackPatchModel> mapped = patchInput.Map<PatchStackInput, StackPatchModel>();
        var result = await mediator.Send(new PatchStack(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackView.Map);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        [FromRoute][Description("Stack ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<StackPatchModel> mapped = patchInput.Map<PatchResourceMetadata, StackPatchModel>();
        var result = await mediator.Send(new PatchStackMetadata(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackView.Map);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameStack(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackView.Map);
    }

    public static async Task<Results<Ok<ContainersDataView>, ProblemHttpResult>> GetContainersData(IMediator mediator, Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainersData(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainersDataView.Map);
    }

    public static async Task<Results<Ok<StackStatsView>, ProblemHttpResult>> GetStats(
        IMediator mediator,
        [Description("The stack id")] Guid stackId,
        [FromQuery][Description("Stats lookback window in hours. Supported values: 24, 48, 72.")] int hours = 24,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetStackStats(stackId, hours), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackStatsView.Map);
    }

    public static async Task<Results<Ok<StackDriftReport>, ProblemHttpResult>> GetDrift(
        IMediator mediator,
        [Description("The stack id")] Guid stackId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStackDrift(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }

    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> UpdateDriftPolicy(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [Description("The stack id")] Guid stackId,
        [FromBody] StackDriftPolicyInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new UpdateStackDriftPolicy(stackId, input.ToPolicy()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, StackView.Map);
    }

    public static async Task<Results<Ok<StackReconciliationResult>, ProblemHttpResult>> Reconcile(
        IMediator mediator,
        [Description("The stack id")] Guid stackId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ReconcileStack(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }

    public static async Task<Results<Ok<ContainerInspectView>, ProblemHttpResult>> InspectContainer(
        IMediator mediator,
        [Description("The stack id")] Guid stackId,
        [Description("The container id")] string containerId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectStackContainer(stackId, containerId), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInspectView.Map);
    }

    public static async IAsyncEnumerable<StackStreamItem> ApplyStack(IMediator mediator, ApplyStackInput applyStackInput, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(applyStackInput.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async IAsyncEnumerable<StackStreamItem> RollbackStack(IMediator mediator, RollbackStackInput rollbackStackInput, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(rollbackStackInput.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Stop(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeStackState(stackIds, StackAction.STOP), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Start(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeStackState(stackIds, StackAction.START), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Pause(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeStackState(stackIds, StackAction.PAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Resume(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeStackState(stackIds, StackAction.UNPAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Restart(IMediator mediator, [FromBody] Guid[] stackIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeStackState(stackIds, StackAction.RESTART), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
