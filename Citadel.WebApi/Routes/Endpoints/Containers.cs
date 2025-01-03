using System.ComponentModel;
using Application.Features.Containers.Commands;
using Application.Features.Containers.Models;
using Application.Features.Containers.Queries;
using Application.Features.Platforms.Commands;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Components.Forms;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Controllers.V1.Resources.Containers;

namespace WebApi.Routes.Endpoints;

public static class Containers
{
    public static async Task<Results<Ok<ContainerInfoView>, ProblemHttpResult>> GetById(IMediator mediator, string id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetContainerById(id), cancellationToken);
        return ControllerExtensions.HandleResult(response, ContainerInfoView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StartContainers(IMediator mediator, [FromBody]string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.START), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StopContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.STOP), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> PauseContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.PAUSE), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> UnpauseContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.UNPAUSE), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RestartContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.RESTART), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new PatchContainers(containersIds, ContainerAction.DELETE), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StreamLogs(IMediator mediator, [FromBody] StreamLogsRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(request.ToCommand(), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<Ok<ContainerStatsView>, ProblemHttpResult>> GetStats(IMediator mediator, [Description("The container id")] string id, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new GetContainerStats(id), cancellationToken);
        return ControllerExtensions.HandleResult(response, ContainerStatsView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> ContainerInfo(IMediator mediator, ContainersInfoRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new UpdateContainersInfo(request), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> ContainerEvent(IMediator mediator, ContainerEventRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new OnContainerEvent(request), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> ContainerLogs(IMediator mediator, ContainerLogRequest request, CancellationToken cancellationToken)
    {
        var response = await mediator.Send(new OnContainerLogs(request), cancellationToken);
        return ControllerExtensions.HandleResultForNoContent(response);
    }
}
