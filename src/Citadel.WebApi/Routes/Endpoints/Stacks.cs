using Application.Features.Stacks.Commands;
using Application.Features.Stacks.Queries;
using Application.Permissions;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Stacks;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.Stacks;

namespace WebApi.Routes.Endpoints;

public static class Stacks
{
    public static async Task<Results<Ok<StackView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStack(stackId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, StackView.Map);
    }

    public static async Task<Results<Ok<StackConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("The stack id")] Guid stackId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetStack(stackId), cancellationToken);
        return EndpointHandlers.HandleResult(result, StackConfigView.Map);
    }

    public static async Task<Results<Ok<StacksView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllStacks(), cancellationToken);
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

    public static async IAsyncEnumerable<DeploymentStreamItem> ApplyDeployment(IMediator mediator, ApplyDeploymentInput applyDeploymentInput, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(applyDeploymentInput.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }
}