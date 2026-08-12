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
using WebApi.Transport;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using System.Runtime.CompilerServices;

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

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Create(
        IMediator mediator,
        [FromBody] CreatePlatformInput request,
        CancellationToken cancellationToken)
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

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(
        IMediator mediator,
        [FromBody] DeletePlatformsInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<ContainersView>, ProblemHttpResult>> ListContainers(IMediator mediator, IPermissionEvaluator permissionService, [Description("The platform id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetContainers(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionService, (containers, perm) => ContainersView.Map(containers, id, perm));
    }

    public static async Task<Results<Ok<PlatformStatsView>, ProblemHttpResult>> GetStats(
        IMediator mediator,
        [Description("The platform id")] Guid id,
        [FromQuery][Description("Stats lookback window in hours. Supported values: 24, 48, 72.")] int hours = 24,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetPlatformStats(id, hours), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformStatsView.Map);
    }

    public static async Task<Results<Ok<PrunePlatformView>, ProblemHttpResult>> Prune(
        IMediator mediator,
        [Description("The platform id")] Guid id,
        [FromBody] PrunePlatformInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, PrunePlatformView.Map);
    }

    public static async Task<Results<Ok<AgentSetupView>, ProblemHttpResult>> GetAgentSetup(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new Application.Features.Platforms.Queries.GetAgentSetup(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AgentSetupView.Map);
    }

    public static async Task<Results<Ok<AgentSetupView>, ProblemHttpResult>> RotateAgentHubKey(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RotateAgentHubKey(), cancellationToken);
        return EndpointHandlers.HandleResult(result, AgentSetupView.Map);
    }

    public static async Task<Results<Ok<EdgeAgentEnrollmentView>, ProblemHttpResult>> CreateEdgeEnrollment(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateEdgeAgentEnrollment(
                id,
                publicEndpoints.EdgeAgentGrpcUrl.AbsoluteUri.TrimEnd('/')),
            cancellationToken);
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

    public static async Task<Results<Ok<SwarmNodeAgentCoverageView>, ProblemHttpResult>> GetNodeAgentCoverage(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetSwarmNodeAgentCoverage(id), cancellationToken);
        if (!result.IsSuccess(out var coverage, out var error))
            return EndpointHandlers.HandleResult(result, static value => SwarmNodeAgentCoverageView.Map(value, false));

        var permission = await permissionEvaluator.EvaluateAsync(id, ResourceType.Platform);
        var canManage = permission.Has(PermissionLevel.Execute, SpecificPermission.ManageNodeAgents);
        return TypedResults.Ok(SwarmNodeAgentCoverageView.Map(coverage, canManage));
    }

    public static IAsyncEnumerable<SwarmNodeAgentProgressItem> InstallNodeAgents(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
        => mediator.CreateStream(
            new InstallSwarmNodeAgents(id, publicEndpoints.EdgeAgentGrpcUrl.AbsoluteUri.TrimEnd('/')),
            cancellationToken);

    public static IAsyncEnumerable<SwarmNodeAgentProgressItem> RepairNodeAgents(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
        => mediator.CreateStream(
            new RepairSwarmNodeAgents(id, publicEndpoints.EdgeAgentGrpcUrl.AbsoluteUri.TrimEnd('/')),
            cancellationToken);

    public static IAsyncEnumerable<SwarmNodeAgentProgressItem> UpgradeNodeAgents(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
        => mediator.CreateStream(
            new UpgradeSwarmNodeAgents(id, publicEndpoints.EdgeAgentGrpcUrl.AbsoluteUri.TrimEnd('/')),
            cancellationToken);

    public static IAsyncEnumerable<SwarmNodeAgentProgressItem> RemoveNodeAgents(
        IMediator mediator,
        [Description("The platform id")] Guid id,
        CancellationToken cancellationToken)
        => mediator.CreateStream(new RemoveSwarmNodeAgents(id), cancellationToken);

}
