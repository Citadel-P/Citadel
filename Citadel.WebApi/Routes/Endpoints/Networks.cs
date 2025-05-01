using System.ComponentModel;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Networks;

namespace WebApi.Routes.Endpoints;

public static class Networks
{
    public static async Task<Results<Ok<NetworksView>, ProblemHttpResult>> List(IMediator mediator, [Description("The platform id")] Guid id, ListNetworksRequest listNetworksRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, NetworkView.Map);
    }

    public static async Task<Results<Ok<NetworkView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateNetworkInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return null; //EndpointHandlers.HandleResult(result, v => v);
    }
}
