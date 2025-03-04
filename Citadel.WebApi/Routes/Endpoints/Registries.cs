using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints;

public static class Registries
{
    public static async Task<Results<Ok<RegistryResponse>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateRegistryRequest request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RegistryResponse.Map);
    }


}
