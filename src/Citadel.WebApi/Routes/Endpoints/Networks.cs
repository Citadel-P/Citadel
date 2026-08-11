using System.ComponentModel;
using Application.Features.Networks.Queries;
using Application.Permissions;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Networks;

namespace WebApi.Routes.Endpoints;

public static class Networks
{
    public static async Task<Results<Ok<NetworksView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("The platform id")] Guid platformId, [AsParameters] ListNetworksRequest listNetworksRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(platformId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, NetworksView.Map);
    }

    public static async Task<Results<Ok<CreateNetworkView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateNetworkInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => new CreateNetworkView(v.NetworkId));
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteNetworksInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<DockerNetworkDetailsView>, ProblemHttpResult>> Inspect(IMediator mediator, IPermissionEvaluator permissionEvaluator, Guid platformId, string networkId, [FromQuery] string? dockerNodeId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectNetwork(platformId, networkId, dockerNodeId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerNetworkDetailsView.Map);
    }
}
