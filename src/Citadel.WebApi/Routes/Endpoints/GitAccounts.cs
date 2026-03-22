using System.ComponentModel;
using Application.Features.GitAccounts.Commands;
using Application.Features.GitAccounts.Queries;
using Domain.Entities.Git;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.GitAccounts;

namespace WebApi.Routes.Endpoints;

public static class GitAccounts
{
    public static async Task<Results<Ok<GitAccountView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] GitAccountInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitAccountView.Map);
    }

    public static async Task<Results<Ok<GitAccountsView>, ProblemHttpResult>> List(IMediator mediator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllGitAccounts(), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitAccountsView.Map);
    }

    public static async Task<Results<Ok<GitAccountView>, ProblemHttpResult>> Get(IMediator mediator, [Description("Git account id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitAccount(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitAccountView.Map);
    }

    public static async Task<Results<Ok<GitAccountConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, [Description("Git account id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetGitAccount(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitAccountConfigView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteGitAccountsInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<GitAccountView>, ProblemHttpResult>> Patch(
        IMediator mediator,
        [FromRoute][Description("Git account ID")] Guid id,
        GitAccountInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<GitAccountInput, GitAccount>();
        var result = await mediator.Send(new PatchGitAccount(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, GitAccountView.Map);
    }
}
