using Application.Features.Auth.Models;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Auth;

namespace WebApi.Routes.Endpoints;

public static class Authentication
{
    public static async Task<Results<Ok<LoginResponse>, ProblemHttpResult>> Login(IMediator mediator, LoginRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(request.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(response);
    }
}
