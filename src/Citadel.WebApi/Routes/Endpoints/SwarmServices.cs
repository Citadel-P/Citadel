using Application.Features.SwarmServices.Commands;
using Application.Features.SwarmServices.Queries;
using Application.Permissions;
using Domain.Contracts.Resources.SwarmServices;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.Runtime.CompilerServices;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.SwarmServices;
using SwarmServiceInspectView = WebApi.Routes.Endpoints.Resources.Swarm.SwarmServiceInspectView;
using SwarmLogsView = WebApi.Routes.Endpoints.Resources.Swarm.SwarmLogsView;

namespace WebApi.Routes.Endpoints;

public static class SwarmServices
{
    public static async Task<Results<Ok<ManagedSwarmServicesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        [FromQuery] Guid? platformId = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetSwarmServices(tags, platformId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, ManagedSwarmServicesView.Map);
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmService(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, ManagedSwarmServiceView.Map);
    }

    public static async Task<Results<Ok<SwarmServiceDuplicateDraftView>, ProblemHttpResult>> GetDuplicateDraft(
        IMediator mediator,
        Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmServiceDuplicateDraft(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, draft => SwarmServiceDuplicateDraftView.Map(draft, id));
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> Create(
        IMediator mediator,
        [FromBody] CreateSwarmServiceInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, ManagedSwarmServiceView.Map);
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> Update(
        IMediator mediator,
        Guid id,
        [FromBody] UpdateSwarmServiceInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ManagedSwarmServiceView.Map);
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        [FromBody] RenameResource input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameSwarmService(input.Id, input.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, ManagedSwarmServiceView.Map);
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> UpdateMetadata(
        IMediator mediator,
        Guid id,
        PatchResourceMetadataDocument input,
        CancellationToken cancellationToken)
    {
        var description = input.Patch.EnumerateObject()
            .FirstOrDefault(static property => string.Equals(property.Name, "description", StringComparison.OrdinalIgnoreCase));
        if (description.Value.ValueKind == System.Text.Json.JsonValueKind.Undefined)
            return TypedResults.Problem(detail: "The metadata patch must contain description.", statusCode: StatusCodes.Status400BadRequest);
        var value = description.Value.ValueKind == System.Text.Json.JsonValueKind.Null
            ? null
            : description.Value.GetString();
        var result = await mediator.Send(new UpdateSwarmServiceMetadata(id, value), cancellationToken);
        return EndpointHandlers.HandleResult(result, ManagedSwarmServiceView.Map);
    }

    public static async Task<Results<Ok<ManagedSwarmServiceView>, ProblemHttpResult>> CheckUpdates(
        IMediator mediator,
        Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CheckSwarmServiceUpdates(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ManagedSwarmServiceView.Map);
    }

    public static async IAsyncEnumerable<SwarmServiceProgressItem> Apply(
        IMediator mediator,
        Guid id,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new ApplySwarmService(id), cancellationToken))
            yield return item;
    }

    public static async IAsyncEnumerable<SwarmServiceProgressItem> Scale(
        IMediator mediator,
        Guid id,
        [FromBody] ScaleSwarmServiceInput input,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new ScaleSwarmService(id, input.Replicas), cancellationToken))
            yield return item;
    }

    public static async IAsyncEnumerable<SwarmServiceProgressItem> ForceUpdate(
        IMediator mediator,
        Guid id,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var item in mediator.CreateStream(new ForceUpdateSwarmService(id), cancellationToken))
            yield return item;
    }

    public static async Task<Results<Ok<SwarmServiceInspectView>, ProblemHttpResult>> Inspect(
        IMediator mediator,
        Guid id,
        CancellationToken cancellationToken) =>
        EndpointHandlers.HandleResult(
            await mediator.Send(new InspectManagedSwarmService(id), cancellationToken),
            SwarmServiceInspectView.Map);

    public static async Task<Results<Ok<SwarmLogsView>, ProblemHttpResult>> GetLogs(
        IMediator mediator,
        Guid id,
        int tail = 100,
        CancellationToken cancellationToken = default) =>
        EndpointHandlers.HandleResult(
            await mediator.Send(new GetManagedSwarmServiceLogs(id, tail), cancellationToken),
            SwarmLogsView.Map);

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(
        IMediator mediator,
        [FromBody] Guid[] ids,
        CancellationToken cancellationToken)
    {
        foreach (var id in ids.Distinct())
        {
            var result = await mediator.Send(new DeleteSwarmService(id), cancellationToken);
            if (result.IsFailure())
                return EndpointHandlers.HandleResultForNoContent(result);
        }
        return TypedResults.NoContent();
    }
}
