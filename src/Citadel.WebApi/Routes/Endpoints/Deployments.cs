using Application.Features.Deployments.Commands;
using Application.Features.Deployments.Queries;
using Domain;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources.Deployments;

namespace WebApi.Routes.Endpoints;

public static class Deployments
{
    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Get(IMediator mediator, [Description("The deployment id")] Guid deploymentId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeployment(deploymentId), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async Task<Results<Ok<DeploymentConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("The deployment id")] Guid deploymentId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetDeployment(deploymentId), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentConfigView.Map);
    }

    public static async Task<Results<Ok<DeploymentsView>, ProblemHttpResult>> List(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllDeployments(), cancellationToken);
        return EndpointHandlers.HandleResult(result, DeploymentsView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] DeploymentInput request, CancellationToken cancellationToken)
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
        var mapped = patchInput.Map<DeploymentInput, Deployment>();
        var result = await mediator.Send(new PatchDeployment(id, mapped), cancellationToken);
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

}
