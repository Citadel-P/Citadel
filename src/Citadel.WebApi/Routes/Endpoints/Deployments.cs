using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Deployments;

namespace WebApi.Routes.Endpoints;

public static class Deployments
{
    public static async Task<Results<Ok<DeploymentsView>, ProblemHttpResult>> List(IMediator mediator, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
        //var result = await mediator.Send(new List(), cancellationToken);
        //return EndpointHandlers.HandleResult(result, DeploymentsView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> GetDeployment(IMediator mediator, [Description("The deployment id")] Guid deploymentId, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();

        //var result = await mediator.Send(new GetDeployment(), cancellationToken);
        //return EndpointHandlers.HandleResult(result, DeploymentView.Map);
    }

    public static async Task<Results<Ok<DeploymentView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] DeploymentInput request, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
        //var result = await mediator.Send(request.ToCommand(), cancellationToken);
        //return EndpointHandlers.HandleResult(result, DeploymentView.Map);
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
        throw new NotImplementedException();
        //var mapped = patchInput.Map<RegistryInput, Registry>();
        //var result = await mediator.Send(new PatchRegistry(id, mapped), cancellationToken);
        //return EndpointHandlers.HandleResult(result, RegistryView.Map);
    }
}
