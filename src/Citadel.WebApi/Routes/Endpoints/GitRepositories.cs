using Application.Features.Deployments.Queries;
using Application.Features.GitRepositories.Commands;
using Application.Features.GitRepositories.Queries;
using Domain.Entities.Git;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.GitRepositories;

namespace WebApi.Routes.Endpoints;

public static class GitRepositories
{
    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] GitRepositoryInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoriesView>, ProblemHttpResult>> List(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllGitRepositories(), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoriesView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryView>, ProblemHttpResult>> Get(IMediator mediator, [Description("Git repository id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepository(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }

    public static async Task<Results<Ok<GitRepositoryConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("Git repository id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitRepository(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryConfigView.Map);
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
        var mapped = patchInput.Map<GitRepositoryInput, GitRepository>();
        var result = await mediator.Send(new PatchGitRepository(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitRepositoryView.Map);
    }
}
