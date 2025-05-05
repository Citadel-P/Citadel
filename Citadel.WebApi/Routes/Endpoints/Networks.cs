using System.ComponentModel;
using Application.Features.Networks.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Networks;

namespace WebApi.Routes.Endpoints;

public static class Networks
{
    public static async Task<Results<Ok<NetworksView>, ProblemHttpResult>> List(IMediator mediator, [Description("The platform id")] Guid id, [AsParameters] ListNetworksRequest listNetworksRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, NetworkView.Map);
    }

    public static async Task<Results<Ok<CreateNetworkView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateNetworkInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => new CreateNetworkView(v.Id));
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteNetworksInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<InspectNetworkView>, ProblemHttpResult>> Inspect(IMediator mediator, Guid platformId, string networkId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectNetwork(platformId, networkId), cancellationToken);
        return EndpointHandlers.HandleResult(result, InspectNetworkView.Map);
    }
}
