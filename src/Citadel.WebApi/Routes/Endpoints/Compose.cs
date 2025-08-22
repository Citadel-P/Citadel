using Domain.Contracts.Resources.Compose;
using Mediator;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Compose;

namespace WebApi.Routes.Endpoints;

public static class Compose
{
    public static async IAsyncEnumerable<ComposeDeploymentEvent> Up(IMediator mediator, [FromBody] ComposeUpRequest composeUpRequest, CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(composeUpRequest.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }
}
