using System.ComponentModel;
using Application.Features.Platforms.Commands;
using Application.Features.Platforms.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;
using Hosting.Common.MergePatch;
using Domain.Entities.Platforms;
using Application.Permissions;

namespace WebApi.Routes.Endpoints;

public static class Platforms
{
    public static async Task<Results<Ok<PlatformsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetPlatforms(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, PlatformsView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetPlatformById(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreatePlatformInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Platform ID")] Guid id,
        PlatformInputPatchDocument patchInput, 
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PlatformInput, Platform>();
        var result = await mediator.Send(new PatchPlatform(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, DeletePlatformsInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ContainersView>, ProblemHttpResult>> ListContainers(IMediator mediator, IPermissionEvaluator permissionService, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainers(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, (containers, perm) => ContainersView.Map(containers, id, perm));
    }

}
