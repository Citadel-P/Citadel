using Application.Features.Features.Lookup.Queries;
using Domain.Contracts.Resources;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Lookup;

namespace WebApi.Routes.Endpoints;

public static class Lookup
{
    public static async Task<Results<Ok<IEnumerable<ResourceInfo>>, ProblemHttpResult>> Get(
        IMediator mediator,
        [AsParameters]
        LookupRequest request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new GetResourceLookupQuery(
                request.SourceResourceType,
                request.SourceResourceId,
                request.TargetResourceType,
                new LookupContext(request.PlatformId)),
            cancellationToken);

        return EndpointHandlers.HandleResult(result, items => items);
    }
}