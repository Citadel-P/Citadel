using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Activities;

namespace WebApi.Routes.Endpoints;

public static class Activities
{
    public static async Task<Results<Ok<ActivitiesView>, ProblemHttpResult>> List(IMediator mediator, [AsParameters] ActivityFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ActivitiesView.Map);
    }
}