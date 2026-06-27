using Application.Features.GitRepositories.Commands;
using Application.Features.GitRepositories.Queries;
using Application.Permissions;
using Domain.Entities.Git;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.GitRepositories;

namespace WebApi.Routes.Endpoints;

public static class GitRepositories
{
    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateGitRepositoryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoriesView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllGitRepositories(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, GitRepositoriesView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Get(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("Git repository id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepository(id), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("Git repository id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepository(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryConfigView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryRefsView>, ProblemHttpResult>> GetRefs(
        IMediator mediator,
        [Description("Git repository id")] Guid id,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepositoryRefs(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryRefsView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryComposeDiscovery>, ProblemHttpResult>> DiscoverComposeProjects(
        IMediator mediator,
        [Description("Git repository id")] Guid id,
        [FromQuery] string? branch,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new DiscoverGitRepositoryComposeProjects(id, branch), cancellationToken);
        return EndpointHandlers.HandleResult(result);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteGitRepositoriesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Git repository ID")] Guid id,
        GitRepositoryInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchGitRepositoryInput, GitRepository>();
        var result = await mediator.Send(new PatchGitRepository(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        [FromRoute][Description("Git repository ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchResourceMetadata, GitRepository>();
        var result = await mediator.Send(new PatchGitRepositoryMetadata(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameGitRepository(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Sync(
        IMediator mediator,
        [FromRoute][Description("Git repository ID")] Guid id,
        [FromQuery] string? branch,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new SyncGitRepository(id, branch), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }
}
