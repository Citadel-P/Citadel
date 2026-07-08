using Application.Features.Deployments.Commands;
using Application.Features.Deployments.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Deployments;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Deployments;

namespace WebApi.Routes.Endpoints;

public static class Deployments
{
    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("Deployment id")] Guid deploymentId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeployment(deploymentId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DeploymentView.Map);
    }

    public static async Task<Results<Ok<DeploymentConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("Deployment id")] Guid deploymentId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeployment(deploymentId), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentConfigView.Map);
    }

    public static async Task<Results<Ok<DeploymentsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetAllDeployments(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DeploymentsView.Map);
    }

    public static async Task<Results<Ok<ContainerInfoView>, ProblemHttpResult>> GetInfo(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeploymentContainerInfo(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInfoView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateDeploymentInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeleteDeployments(deploymentIds), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Deployment ID")] Guid id,
        DeploymentInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchDeploymentInput, Deployment>();
        var result = await mediator.Send(new PatchDeployment(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        [FromRoute][Description("Deployment ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchResourceMetadata, Deployment>();
        var result = await mediator.Send(new PatchDeploymentMetadata(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Rename(IMediator mediator, [FromBody] RenameResource renameResource, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameDeployment(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async IAsyncEnumerable<DeploymentStreamItem> ApplyDeployment(IMediator mediator, ApplyDeploymentInput applyDeploymentInput, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(applyDeploymentInput.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Resume(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeDeploymentState(deploymentIds, DeploymentAction.UNPAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Pause(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeDeploymentState(deploymentIds, DeploymentAction.PAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
    public static async Task<Results<NoContent, ProblemHttpResult>> Restart(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeDeploymentState(deploymentIds, DeploymentAction.RESTART), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Stop(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeDeploymentState(deploymentIds, DeploymentAction.STOP), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Start(IMediator mediator, [FromBody] Guid[] deploymentIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ChangeDeploymentState(deploymentIds, DeploymentAction.START), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ContainerInspectView>, ProblemHttpResult>> Inspect(IMediator mediator, [Description("Deployment id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectDeployment(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInspectView.Map);
    }

    public static async Task<Results<Ok<ContainerStatsView>, ProblemHttpResult>> GetStats(
        IMediator mediator,
        [Description("Deployment id")] Guid id,
        [FromQuery][Description("Stats lookback window in hours. Supported values: 24, 48, 72.")] int hours = 24,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetDeploymentStats(id, hours), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerStatsView.Map);
    }
}
