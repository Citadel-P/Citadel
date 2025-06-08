using System.ComponentModel;
using Application.Features.Platforms.Commands;
using Application.Features.Platforms.Queries;
using Application.Features.Platforms.Queries.Models;
using Hosting.Extensions;
using Infrastructure.Entities;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints;

public static class Platforms
{
    public static async Task<Results<Ok<PlatformsView>, ProblemHttpResult>> List(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatforms(), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformsView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> GetById(IMediator mediator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatformById(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> GetInfo(IMediator mediator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatformInfo(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] PlatformInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Platform id")] Guid id,
        PlatformInputPatchDocument patchInput, 
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PlatformInput, Platform>();
        var result = await mediator.Send(new PatchPlatform(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DeletePlatform(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ContainersView>, ProblemHttpResult>> ListContainers(IMediator mediator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainers(new GetContainersQuery(All: true), id), cancellationToken);
        return EndpointHandlers.HandleResult(result, ContainersView.Map);
    }

}
