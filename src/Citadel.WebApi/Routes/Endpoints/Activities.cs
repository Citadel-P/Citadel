using Application.Features.Activities.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Activities;

namespace WebApi.Routes.Endpoints;

public static class Activities
{
    public static async Task<Results<Ok<ActivityView>, ProblemHttpResult>> Get(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetActivity(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ActivityView.Map);
    }

    public static async Task<Results<Ok<ActivitiesView>, ProblemHttpResult>> List(IMediator mediator, [AsParameters] ActivityFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ActivitiesView.Map);
    }
}