using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
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
}
