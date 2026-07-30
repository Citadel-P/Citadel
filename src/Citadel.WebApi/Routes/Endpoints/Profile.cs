using Application.Features.Identity.Profile.Commands;
using Application.Features.Identity.Profile.Queries;
using Application.Permissions;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Identity.Profile;

namespace WebApi.Routes.Endpoints;

public static class Profile
{
    public static async Task<Results<Ok<CurrentProfileView>, ProblemHttpResult>> Get(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetCurrentProfile(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, CurrentProfileView.Map);
    }

    public static async Task<Results<Ok<CurrentProfileView>, ProblemHttpResult>> Update(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] UpdateCurrentProfileInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, CurrentProfileView.Map);
    }

    public static async Task<Results<Ok<UserPreferencesView>, ProblemHttpResult>> GetPreferences(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetUserPreferences(), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserPreferencesView.Map);
    }

    public static async Task<Results<Ok<UserPreferencesView>, ProblemHttpResult>> PatchPreferences(
        IMediator mediator,
        PatchUserPreferencesInputPatchDocument patchInput,
        CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<PatchUserPreferencesModel> mapped = patchInput.Map<PatchUserPreferencesInput, PatchUserPreferencesModel>();
        var result = await mediator.Send(new PatchUserPreferences(mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserPreferencesView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> ChangePassword(
        IMediator mediator,
        [FromBody] ChangeCurrentPasswordInput input,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<UserSessionsView>, ProblemHttpResult>> ListSessions(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new ListUserSessions(), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserSessionsView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> RevokeSession(
        IMediator mediator,
        [FromRoute] Guid sessionId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RevokeUserSession(sessionId), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    public static async Task<Results<Ok<RevokeOtherProfileSessionsView>, ProblemHttpResult>> RevokeOtherSessions(
        IMediator mediator,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RevokeOtherUserSessions(), cancellationToken);
        return EndpointHandlers.HandleResult(result, value => new RevokeOtherProfileSessionsView(value.Count));
    }
}
