using Application.Features.Platforms.Commands;
using Application.Features.Platforms.Queries;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Hosting.Extensions;
using Application.Features.Platforms.Queries.Models;
using System.ComponentModel.DataAnnotations;
using WebApi.Controllers.V1.Resources.Platforms;
using WebApi.Controllers.V1.Resources.Containers;
using Application.Features.Platforms.Models;

namespace WebApi.Controllers.V1;

/// <summary>
/// Manages the docker containers requests
/// </summary>
[Authorize]
[ApiController]
[Produces("application/json")]
[Route("api/v{version:apiVersion}/[controller]")]
public sealed class PlatformsController(IMediator mediator) : ControllerBase
{
    /// <summary>
    /// List all platforms
    /// </summary>
    [HttpGet]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult<PlatformsView>> List(CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetPlatforms(), cancellationToken);
        return this.HandleResult(response, PlatformsView.Map);
    }

    /// <summary>
    /// Get platform by Id
    /// </summary>
    /// <param name="id">The platform id</param>
    /// <param name="cancellationToken"></param>
    [HttpGet("{id}")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status404NotFound)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult<PlatformView>> GetById([Required] Guid id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetPlatformById(id), cancellationToken);
        return this.HandleResult(response, PlatformView.Map);
    }

    /// <summary>
    /// Get platform info from agent
    /// </summary>
    /// <param name="id">The platform id</param>
    /// <param name="cancellationToken"></param>
    [HttpGet("{id}/info")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status404NotFound)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult<PlatformView>> GetInfo([Required] Guid id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetPlatformInfo(id), cancellationToken);
        return this.HandleResult(response, PlatformView.Map);
    }

    /// <summary>
    /// Create or update a platform
    /// </summary>
    /// <param name="platformParams"></param>
    /// <param name="cancellationToken"></param>
    [HttpPut]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status409Conflict)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult<PlatformView>> Put([FromBody] PutPlatformRequest platformParams, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(platformParams.ToCommand(), cancellationToken);
        return this.HandleResult(response, PlatformView.Map);
    }

    /// <summary>
    /// Delete a platform
    /// </summary>
    /// <param name="id">The platform Id</param>
    /// <param name="cancellationToken"></param>
    /// <returns></returns>
    [HttpDelete("{id}")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status404NotFound)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult> Delete([Required] Guid id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new DeletePlatform(id), cancellationToken);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// returns the list of containers of the given platform
    /// </summary>
    /// <param name="id">The platform id</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpGet("{id}/containers")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult<ContainersInfoView>> ListContainers([Required] Guid id, CancellationToken cancellation)
    {
        var response = await mediator.Send(new GetContainers(new GetContainersQuery(All: true), id), cancellation);
        return this.HandleResult(response, ContainersInfoView.Map);
    }

    /// <summary>
    /// Updates a new system info entry
    /// /// </summary>
    /// <param name="systemInfo">The system info payload</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPut("_info")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> SystemInfo(SystemInfoRequest systemInfo, CancellationToken cancellation)
    {
        var response = await mediator.Send(new UpdateSystemInfo(systemInfo), cancellation);
        return this.HandleResultForNoContent(response);
    }

}