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
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Application.Permissions;
using Microsoft.AspNetCore.Http.Extensions;

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

    public static async Task<Results<Ok<EdgeAgentEnrollmentView>, ProblemHttpResult>> CreateEdgeEnrollment(
        IMediator mediator,
        HttpContext httpContext,
        [FromServices] IConfiguration configuration,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
    {
        var coreUrl = GetEdgeAgentGrpcUrl(httpContext, configuration);
        var result = await mediator.Send(new CreateEdgeAgentEnrollment(id, coreUrl), cancellationToken);
        return EndpointHandlers.HandleResult(result, EdgeAgentEnrollmentView.Map);
    }

    public static async Task<Results<Ok<EdgeAgentStatusView>, ProblemHttpResult>> GetEdgeStatus(
        IMediator mediator,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetEdgeAgentStatus(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, EdgeAgentStatusView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RevokeEdge(
        IMediator mediator,
        IEdgeAgentSessionTerminator sessionTerminator,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RevokeEdgeAgent(id), cancellationToken);
        if (result.IsSuccess())
        {
            sessionTerminator.Disconnect(id, "Edge Agent binding was revoked.");
        }

        return EndpointHandlers.HandleResultForNoContent(result);
    }

    private static string GetEdgeAgentGrpcUrl(HttpContext httpContext, IConfiguration configuration)
    {
        var configuredUrl = configuration["EdgeAgent:PublicGrpcUrl"];
        if (!string.IsNullOrWhiteSpace(configuredUrl))
        {
            return configuredUrl.TrimEnd('/');
        }

        return UriHelper.BuildAbsolute(httpContext.Request.Scheme, httpContext.Request.Host).TrimEnd('/');
    }

}
