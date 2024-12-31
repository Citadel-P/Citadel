using Asp.Versioning;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Hosting.Extensions;
using System.ComponentModel.DataAnnotations;
using WebApi.Controllers.V1.Resources.Containers;
using Application.Features.Containers.Queries;
using Application.Features.Platforms.Commands;
using Application.Features.Containers.Models;
using Application.Features.Containers.Commands;

namespace WebApi.Controllers.V1;

/// <summary>
/// Manages the docker containers requests
/// </summary>
[Authorize]
[ApiController]
[ApiVersion("1.0")]
[Produces("application/json")]
[Route("api/v{version:apiVersion}/[controller]")]
public sealed class ContainersController(IMediator mediator) : ControllerBase
{
    /// <summary>
    /// Get container by Id
    /// </summary>
    /// <param name="id">The container id</param>
    /// <param name="cancellationToken"></param>
    [HttpGet("{id}")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status404NotFound)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    public async Task<ActionResult<ContainerInfoView>> GetById([Required] string id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetContainerById(id), cancellationToken);
        return this.HandleResult(response, ContainerInfoView.Map);
    }

    /// <summary>
    /// Start the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("start")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> StartContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.START), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Stop the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("stop")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> StopContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.STOP), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Pause the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("pause")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> PauseContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.PAUSE), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Restart the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("restart")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> RestartContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.RESTART), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Unpause the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("unpause")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> UnpauseContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.UNPAUSE), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Delete the given container(s)
    /// </summary>
    /// <param name="containersIds">List of containers ids</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPatch("delete")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> DeleteContainers([Required][FromBody] string[] containersIds, CancellationToken cancellation)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.DELETE), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Request to start (or stop) streaming container logs
    /// </summary>
    /// <param name="request">The request params</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPost("stream-logs")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> StreamLogs([FromBody] StreamLogsRequest request, CancellationToken cancellation)
    {
        var response = await mediator.Send(request.ToCommand(), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Get container stats
    /// </summary>
    /// <param name="id">The container id</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpGet("{id}/stats")]
    [ProducesResponseType(StatusCodes.Status200OK)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult<ContainerStatsView>> GetStats([Required] string id, CancellationToken cancellation)
    {
        var response = await mediator.Send(new GetContainerStats(id), cancellation);
        return this.HandleResult(response, ContainerStatsView.Map);
    }

    /// <summary>
    /// Update or create the containers info entry
    /// </summary>
    /// <param name="request">The container list payload</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPut("_info")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> ContainerInfo(ContainersInfoRequest request, CancellationToken cancellation)
    {
        var response = await mediator.Send(new UpdateContainersInfo(request), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Handles the event issued from a container
    /// </summary>
    /// <param name="request">The container event payload</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPut("_event")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> ContainerEvent(ContainerEventRequest request, CancellationToken cancellation)
    {
        var response = await mediator.Send(new OnContainerEvent(request), cancellation);
        return this.HandleResultForNoContent(response);
    }

    /// <summary>
    /// Request to start (or stop) streaming container logs
    /// </summary>
    /// <param name="request">The request params</param>
    /// <param name="cancellation"></param>
    /// <returns></returns>
    [HttpPost("_logs")]
    [ProducesResponseType(StatusCodes.Status204NoContent)]
    [ProducesResponseType(typeof(ValidationProblemDetails), StatusCodes.Status400BadRequest)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status403Forbidden)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status401Unauthorized)]
    [ProducesResponseType(typeof(ProblemDetails), StatusCodes.Status502BadGateway)]
    public async Task<ActionResult> ContainerLogs(ContainerLogRequest request, CancellationToken cancellation)
    {
        var response = await mediator.Send(new OnContainerLogs(request), cancellation);
        return this.HandleResultForNoContent(response);
    }
}