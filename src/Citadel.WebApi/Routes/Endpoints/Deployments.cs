using Application.Features.Deployments.Queries;
using Domain.Entities;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Deployments;
using Hosting.Common.MergePatch;
using Application.Features.Deployments.Commands;

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

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteDeploymentsInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
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
}
