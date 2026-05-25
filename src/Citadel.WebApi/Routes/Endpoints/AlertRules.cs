using Application.Features.Alerters.Commands;
using Application.Features.Alerters.Queries;
using Application.Permissions;
using Domain.Entities.Alerts;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Alerters;

namespace WebApi.Routes.Endpoints;

public static class AlertRules
{
    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> GetRule(IMediator mediator, IPermissionEvaluator permissionService, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertRule(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertRuleConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertRule(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleConfigView.Map);
    }

    public static async Task<Results<Ok<AlertRulesView>, ProblemHttpResult>> ListRules(IMediator mediator, IPermissionEvaluator permissionService, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertRules(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, AlertRulesView.Map);
    }

    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> CreateRule(IMediator mediator, [FromBody] CreateAlertRuleInput request, CancellationToken cancellationToken)
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
        var mapped = patchInput.Map<PatchAlertRuleInput, AlertRule>();
        var result = await mediator.Send(new PatchAlertRule(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> PatchRuleMetadata(
        IMediator mediator,
        [FromRoute][Description("Alert rule ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchResourceMetadata, AlertRule>();
        var result = await mediator.Send(new PatchAlertRuleMetadata(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertRuleView>, ProblemHttpResult>> RenameRule(
        IMediator mediator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameAlertRule(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertRuleView.Map);
    }

    public static async Task<Results<Ok<AlertChannelView>, ProblemHttpResult>> GetChannel(IMediator mediator, Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannel(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelView.Map);
    }

    public static async Task<Results<Ok<AlertChannelView>, ProblemHttpResult>> CreateChannel(IMediator mediator, [FromBody] AlertChannelInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> VerifyChannel(IMediator mediator, [FromBody] VerifyAlertChannelInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteChannels(IMediator mediator, [FromBody] DeleteAlertChannelsInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<AlertChannelView>, ProblemHttpResult>> PatchChannel(
        IMediator mediator,
        [FromRoute][Description("Alert channel ID")] Guid id,
        AlertChannelInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<AlertChannelInput, AlertChannel>();
        var result = await mediator.Send(new PatchAlertChannel(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelView.Map);
    }

    public static async Task<Results<Ok<AlertChannelsView>, ProblemHttpResult>> ListChannels(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAlertChannels(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AlertChannelsView.Map);
    }
}