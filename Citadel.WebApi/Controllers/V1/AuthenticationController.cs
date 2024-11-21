using Application.Features.Auth.Models;
using Asp.Versioning;
using Mediator;
using Microsoft.AspNetCore.Mvc;
using Hosting.Extensions;
using WebApi.Controllers.V1.Resources.Auth;

namespace WebApi.Controllers.V1;

/// <summary>
/// The Authentication controller
/// </summary>
[ApiController]
[ApiVersion("1.0")]
[Produces("application/json")]
[Route("api/v{version:apiVersion}/[controller]")]
public sealed class AuthenticationController(IMediator mediator) : ControllerBase
{
    /// <summary>
    /// Check user credentials and issue a jwt token on successful login
    /// </summary>
    [HttpPost("login")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status404NotFound)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    public async Task<ActionResult<LoginResponse>> Login(LoginRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(request.ToQuery(), cancellationToken);
        return this.HandleResult(response);
    }
}