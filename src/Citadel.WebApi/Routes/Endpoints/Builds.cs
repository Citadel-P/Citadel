using Application.Features.Builds.Commands;
using Application.Features.Builds.Queries;
using Application.Models;
using Application.Permissions;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Builds;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Transport;

namespace WebApi.Routes.Endpoints;

public static class Builds
{
    public static async Task<Results<Ok<BuildProjectsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBuildProjects(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectsView.Map);
    }

    public static async Task<Results<Ok<BuildProjectView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build project ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildProject(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectView.Map);
    }

    public static async Task<Results<Ok<BuildProjectView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] BuildProjectInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateBuildProject(input.ToModel()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectView.Map);
    }

    public static async Task<Results<Ok<BuildProjectView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build project ID")] Guid id,
        UpdateBuildProjectInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateBuildProjectInput(),
            ApplicationJsonContext.Default.UpdateBuildProjectInput);

        var result = await mediator.Send(
            new UpdateBuildProject(
                id,
                input.ToModel(),
                patchInput.ContainsProperty("description"),
                patchInput.ContainsProperty("buildArgs"),
                patchInput.ContainsProperty("buildSecrets"),
                patchInput.ContainsProperty("webhook")),
            cancellationToken);

        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectView.Map);
    }

    public static async Task<Results<Ok<BuildProjectView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameBuildProject(renameResource.Id, renameResource.Name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectView.Map);
    }

    public static async Task<Results<Ok<BuildProjectView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build project ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new PatchResourceMetadata(string.Empty, []),
            ApplicationJsonContext.Default.PatchResourceMetadata);

        var result = await mediator.Send(new PatchBuildProjectMetadata(id, input.Description), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildProjectView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Archive(
        IMediator mediator,
        [FromRoute][Description("Build project ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ArchiveBuildProject(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<BuildRunView>, ProblemHttpResult>> QueueRun(
        IMediator mediator,
        [FromRoute][Description("Build project ID")] Guid id,
        [FromBody] QueueBuildRunInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new QueueBuildRun(id, input.ToModel()), cancellationToken);
        return EndpointHandlers.HandleResult(result, BuildRunView.Map);
    }
}

public static class BuildRuns
{
    public static async Task<Results<Ok<BuildRunsView>, ProblemHttpResult>> List(
        IMediator mediator,
        [FromQuery] Guid? projectId = null,
        [FromQuery] int? limit = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBuildRuns(projectId, limit ?? 50), cancellationToken);
        return EndpointHandlers.HandleResult(result, BuildRunsView.Map);
    }

    public static async Task<Results<Ok<BuildRunView>, ProblemHttpResult>> Get(
        IMediator mediator,
        [FromRoute][Description("Build run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildRun(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BuildRunView.Map);
    }

    public static async Task<Results<Ok<BuildLogsView>, ProblemHttpResult>> GetLogs(
        IMediator mediator,
        [FromRoute][Description("Build run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildRunLogs(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, BuildLogsView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Cancel(
        IMediator mediator,
        [FromRoute][Description("Build run ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CancelBuildRun(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}

public static class BuildAgentPools
{
    public static async Task<Results<Ok<BuildAgentPoolsView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromQuery] string[]? tags = null,
        CancellationToken cancellationToken = default)
    {
        var result = await mediator.Send(new GetBuildAgentPools(tags), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolsView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildAgentPool(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] BuildAgentPoolInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new CreateBuildAgentPool(input.ToModel()), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build pool ID")] Guid id,
        UpdateBuildAgentPoolInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new UpdateBuildAgentPoolInput(),
            ApplicationJsonContext.Default.UpdateBuildAgentPoolInput);

        var result = await mediator.Send(
            new UpdateBuildAgentPool(id, input.ToModel(), patchInput.ContainsProperty("description")),
            cancellationToken);

        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameBuildAgentPool(renameResource.Id, renameResource.Name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build pool ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var input = patchInput.ApplyTo(
            new PatchResourceMetadata(string.Empty, []),
            ApplicationJsonContext.Default.PatchResourceMetadata);

        var result = await mediator.Send(new PatchBuildAgentPoolMetadata(id, input.Description), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<BuildAgentPoolView>, ProblemHttpResult>> Test(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new TestBuildAgentPool(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, BuildAgentPoolView.Map);
    }

    public static async Task<Results<Ok<EdgeAgentEnrollmentView>, ProblemHttpResult>> CreateEdgeEnrollment(
        IMediator mediator,
        ICitadelPublicEndpoints publicEndpoints,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new CreateBuildAgentPoolEdgeEnrollment(
                id,
                publicEndpoints.EdgeAgentGrpcUrl.AbsoluteUri.TrimEnd('/')),
            cancellationToken);
        return EndpointHandlers.HandleResult(result, EdgeAgentEnrollmentView.Map);
    }

    public static async Task<Results<Ok<EdgeAgentStatusView>, ProblemHttpResult>> GetEdgeStatus(
        IMediator mediator,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetBuildAgentPoolEdgeStatus(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, EdgeAgentStatusView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RevokeEdge(
        IMediator mediator,
        IEdgeAgentSessionTerminator sessionTerminator,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RevokeBuildAgentPoolEdgeAgent(id), cancellationToken);
        if (result.IsSuccess())
        {
            sessionTerminator.Disconnect(EdgeAgentResourceType.BuildAgentPool, id, "Build pool Edge Agent binding was revoked.");
        }

        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Archive(
        IMediator mediator,
        [FromRoute][Description("Build pool ID")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ArchiveBuildAgentPool(id), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

}
