using Application.Features.Identity.Auth.Commands;
using Application.Features.Identity.Auth.Models;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Identity.Auth;

namespace WebApi.Routes.Endpoints;

public static class Authentication
{
    public static async Task<Results<Ok<LoginResponse>, ProblemHttpResult>> Login(IMediator mediator, LoginRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(request.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Logout(IMediator mediator, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new LogoutCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(response);
    }

    public static async Task<Results<Ok<RefreshTokenResponse>, ProblemHttpResult>> RefreshToken(IMediator mediator, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new RefreshTokenCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(response, (token) => new RefreshTokenResponse(token));
    }
}
