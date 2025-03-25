using System.ComponentModel;
using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using Application.Features.Containers.Commands;
using Application.Features.Containers.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Routes.Endpoints;

public static class Containers
{
    public static async Task<Results<Ok<ContainerInfoView>, ProblemHttpResult>> GetById(IMediator mediator, string id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainerById(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInfoView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StartContainers(IMediator mediator, [FromBody]string[] containersIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainers(containersIds, ContainerAction.START), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StopContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainers(containersIds, ContainerAction.STOP), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> PauseContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainers(containersIds, ContainerAction.PAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> UnpauseContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainers(containersIds, ContainerAction.UNPAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RestartContainers(IMediator mediator, [FromBody] string[] containersIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainers(containersIds, ContainerAction.RESTART), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteContainers(IMediator mediator, [FromBody] DeleteContainersRequest request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async IAsyncEnumerable<ContainerLogReply> StreamLogs(IMediator mediator, [FromBody] StreamLogsRequest request, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var reply in mediator.CreateStream(request.ToCommand(), cancellationToken))
        {
            yield return reply;
        }
    }

    public static async Task<Results<Ok<ContainerStatsView>, ProblemHttpResult>> GetStats(IMediator mediator, [Description("The container id")] string id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainerStats(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerStatsView.Map);
    }

    public static async Task<Results<Ok<ContainerInspectView>, ProblemHttpResult>> Inspect(IMediator mediator, [Description("The container id")] string id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectContainer(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInspectView.Map);
    }
}
