using Application.Features.Swarm.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Swarm;

namespace WebApi.Routes.Endpoints;

public static class SwarmNodes
{
    public static async Task<Results<Ok<SwarmNodesView>, ProblemHttpResult>> List(
        IMediator mediator,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmNodes(platformId), cancellationToken);
        return EndpointHandlers.HandleResult(result, SwarmNodesView.Map);
    }

    public static async Task<Results<Ok<SwarmNodeView>, ProblemHttpResult>> Get(
        IMediator mediator,
        Guid platformId,
        string nodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmNode(platformId, nodeId), cancellationToken);
        return EndpointHandlers.HandleResult(result, SwarmNodeView.Map);
    }
}
