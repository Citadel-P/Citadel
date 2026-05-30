using Application.Features.Identity.Teams.Commands;
using Application.Features.Identity.Teams.Queries;
using Application.Permissions;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Identity.Teams;

namespace WebApi.Routes.Endpoints;

public static class Teams
{
    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateTeamInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamsView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, [AsParameters] TeamsFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, TeamsView.Map);
    }

    public static async Task<Results<Ok<IEnumerable<TeamSearchItemView>>, ProblemHttpResult>> Search(IMediator mediator, [AsParameters] TeamSearchFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, items => items.Select(TeamSearchItemView.Map));
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> Get(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetTeam(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> Patch(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, PatchTeamInputPatchDocument patchInput, CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<PatchTeamModel> mapped = patchInput.Map<PatchTeamInput, PatchTeamModel>();
        var result = await mediator.Send(new PatchTeam(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> AddRole(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromBody] AddTeamRoleInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> RemoveRole(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromRoute][Description("Role ID")] Guid roleId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RemoveTeamRole(id, roleId), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> AddMember(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromBody] AddTeamMemberInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> RemoveMember(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromRoute][Description("User ID")] Guid userId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RemoveTeamMember(id, userId), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> AddResourceAccess(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromBody] AddTeamResourceAccessInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> RemoveResourceAccess(IMediator mediator, [FromRoute][Description("Team ID")] Guid id, [FromBody] RemoveTeamResourceAccessInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<Ok<TeamView>, ProblemHttpResult>> Rename(IMediator mediator, [FromBody] RenameResource renameResource, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameTeam(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, TeamView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteTeamsInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
