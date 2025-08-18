using System.ComponentModel;
using Application.Features.Containers.Commands;
using Application.Features.Containers.Queries;
using Application.Permissions;
using Domain;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Routes.Endpoints;

public static class Containers
{
    public static async Task<Results<Ok<ContainerView>, ProblemHttpResult>> GetById(IMediator mediator, IContainerPermissionService permissionService, string id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainerById(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, ContainerView.Map);
    }

    public static async Task<Results<Ok<ContainerInfoView>, ProblemHttpResult>> GetInfo(IMediator mediator, string id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainerInfoById(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainerInfoView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StartContainers(IMediator mediator, [FromBody]string[] containerIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainer(containerIds, ContainerAction.START), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> StopContainers(IMediator mediator, [FromBody] string[] containerIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainer(containerIds, ContainerAction.STOP), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> PauseContainers(IMediator mediator, [FromBody] string[] containerIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainer(containerIds, ContainerAction.PAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> UnpauseContainers(IMediator mediator, [FromBody] string[] containerIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainer(containerIds, ContainerAction.UNPAUSE), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RestartContainers(IMediator mediator, [FromBody] string[] containerIds, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new PatchContainer(containerIds, ContainerAction.RESTART), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteContainers(IMediator mediator, [FromBody] DeleteContainersRequest request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
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

    public static async Task<Results<Ok<CreateContainerView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateContainerInput request, CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, v => new CreateContainerView(v));
    }
}
