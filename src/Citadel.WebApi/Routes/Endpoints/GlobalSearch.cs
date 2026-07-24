using Application.Features.Search.Models;
using Application.Features.Search.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Search;

namespace WebApi.Routes.Endpoints;

public static class GlobalSearch
{
    public static async Task<Results<Ok<GlobalSearchResponse>, ProblemHttpResult>> Get(
        IMediator mediator,
        [AsParameters] GlobalSearchRequest request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new GetGlobalSearchQuery(
                request.Query,
                request.Types,
                request.LimitPerType),
            cancellationToken);

        return EndpointHandlers.HandleResult(result);
    }
}
