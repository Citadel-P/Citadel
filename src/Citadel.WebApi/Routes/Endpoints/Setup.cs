using Application.Features.Identity.Auth.Models;
using Application.Features.Identity.Setup;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Identity.Setup;

namespace WebApi.Routes.Endpoints;

public static class Setup
{
    public static async Task<Results<Ok<SetupStatusView>, ProblemHttpResult>> GetStatus(
        HttpResponse response,
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        response.Headers.CacheControl = "no-store";
        var result = await mediator.Send(new GetSetupStatus(), cancellationToken);
        return EndpointHandlers.HandleResult(result, SetupStatusView.Map);
    }

    public static async Task<Results<Ok<LoginResponse>, ProblemHttpResult>> Initialize(
        IMediator mediator,
        InitializeCitadelInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }
}
