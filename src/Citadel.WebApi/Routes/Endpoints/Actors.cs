using Application.Features.Actors.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Actors;

namespace WebApi.Routes.Endpoints;

public static class Actors
{
    public static async Task<Results<Ok<ActorView>, ProblemHttpResult>> Get(
        IMediator mediator,
        [FromRoute][Description("Actor ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetActor(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ActorView.Map);
    }

    public static async Task<Results<Ok<ActorView>, ProblemHttpResult>> PatchEnabled(
        IMediator mediator,
        [FromRoute][Description("Actor ID")] Guid id,
        [FromBody] PatchActorEnabledInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ActorView.Map);
    }
}
