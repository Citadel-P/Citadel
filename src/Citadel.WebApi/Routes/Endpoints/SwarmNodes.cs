using Application.Features.Swarm.Queries;
using Application.Permissions;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Swarm;

namespace WebApi.Routes.Endpoints;

public static class SwarmNodes
{
    public static async Task<Results<Ok<SwarmNodesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmNodes(platformId), cancellationToken);
        return await EndpointHandlers.HandleResult(
            result,
            permissionEvaluator,
            (nodes, evaluator) => SwarmNodesView.Map(nodes, platformId, evaluator));
    }

    public static async Task<Results<Ok<SwarmNodeView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string nodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmNode(platformId, nodeId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, SwarmNodeView.Map);
    }

    public static async Task<Results<Ok<SwarmNodeInspectView>, ProblemHttpResult>> Inspect(
        IMediator mediator,
        Guid platformId,
        string nodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectSwarmNode(platformId, nodeId), cancellationToken);
        return EndpointHandlers.HandleResult(result, SwarmNodeInspectView.Map);
    }
}
