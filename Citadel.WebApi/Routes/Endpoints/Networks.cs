using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Networks;

namespace WebApi.Routes.Endpoints;

public static class Networks
{
    public static async Task<Results<Ok<NetworkView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateNetworkInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => new NetworkView());
    }
}
