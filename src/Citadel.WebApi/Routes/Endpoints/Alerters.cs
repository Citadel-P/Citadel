using Application.Features.Alerters.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Alerters;

namespace WebApi.Routes.Endpoints;

public static class Alerters
{
    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> GetRule(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertRule(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertRulesView>, ProblemHttpResult>> ListRules(IMediator mediator, [AsParameters] AlertRuleFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRulesView.Map);
    }

    public static async Task<Results<Ok<AlertChannelView>, ProblemHttpResult>> GetChannel(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannel(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelView.Map);
    }

    public static async Task<Results<Ok<IEnumerable<AlertChannelView>>, ProblemHttpResult>> ListChannels(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannels(), cancellationToken);
        return EndpointHandlers.HandleResult(result, channels => channels.Select(AlertChannelView.Map));
    }
}