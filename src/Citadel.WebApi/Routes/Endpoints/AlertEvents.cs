
using Application.Features.Alerters.Commands;
using Application.Features.Alerters.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Alerters;

namespace WebApi.Routes.Endpoints;

public static class AlertEvents
{
    public static async Task<Results<Ok<AlertEventView>, ProblemHttpResult>> Get(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertEvent(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertEventView.Map);
    }

    public static async Task<Results<Ok<AlertEventsView>, ProblemHttpResult>> List(IMediator mediator, [AsParameters] AlertEventFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertEventsView.Map);
    }

    public static async Task<Results<Ok<UnresolvedAlertsCountView>, ProblemHttpResult>> GetUnresolvedCount(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetUnresolvedAlertEventsCount(), cancellationToken);
        return EndpointHandlers.HandleResult(result, UnresolvedAlertsCountView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Acknowledge(
        IMediator mediator,
        [FromBody] AcknowledgeAlertEventsInput request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Resolve(
        IMediator mediator,
        [FromBody] ResolveAlertEventsInput request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
