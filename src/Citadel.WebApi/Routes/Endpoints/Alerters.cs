using System.ComponentModel;
using Application.Features.Alerters.Commands;
using Application.Features.Alerters.Queries;
using Domain.Entities.Alerts;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
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

    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> CreateRule(IMediator mediator, [FromBody] AlertRuleInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteRules(IMediator mediator, [FromBody] DeleteAlertRulesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> PatchRule(
        IMediator mediator,
        [FromRoute][Description("Alert rule ID")] Guid id,
        AlertRuleInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<AlertRuleInput, AlertRule>();
        var result = await mediator.Send(new PatchAlertRule(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertChannelView>, ProblemHttpResult>> GetChannel(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannel(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelView.Map);
    }

    public static async Task<Results<Ok<AlertChannelsView>, ProblemHttpResult>> ListChannels(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannels(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelsView.Map);
    }
}