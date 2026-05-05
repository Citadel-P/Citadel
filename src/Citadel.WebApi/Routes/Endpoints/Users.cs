using Application.Features.Identity.Users.Commands;
using Application.Features.Identity.Users.Queries;
using Domain.Contracts.Resources.Identity;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Identity.Users;

namespace WebApi.Routes.Endpoints;

public static class Users
{
    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] CreateUserInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UsersView>, ProblemHttpResult>> List(IMediator mediator, [AsParameters] UsersFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, UsersView.Map);
    }

    public static async Task<Results<Ok<IEnumerable<UserSearchItemView>>, ProblemHttpResult>> Search(IMediator mediator, [AsParameters] UserSearchFilter filter, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(filter.ToQuery(), cancellationToken);
        return EndpointHandlers.HandleResult(result, items => items.Select(UserSearchItemView.Map));
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> Get(IMediator mediator, [FromRoute][Description("User ID")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetUser(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> Patch(IMediator mediator, [FromRoute][Description("User ID")] Guid id, PatchUserInputPatchDocument patchInput, CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<PatchUserModel> mapped = patchInput.Map<PatchUserInput, PatchUserModel>();
        var result = await mediator.Send(new PatchUser(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> AddRole(IMediator mediator, [FromRoute][Description("User ID")] Guid id, [FromBody] AddUserRoleInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> RemoveRole(IMediator mediator, [FromRoute][Description("User ID")] Guid id, [FromRoute][Description("Role ID")] Guid roleId, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RemoveUserRole(id, roleId), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> AddResourceAccess(IMediator mediator, [FromRoute][Description("User ID")] Guid id, [FromBody] AddUserResourceAccessInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> RemoveResourceAccess(IMediator mediator, [FromRoute][Description("User ID")] Guid id, [FromBody] RemoveUserResourceAccessInput input, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(input.ToCommand(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<Ok<UserView>, ProblemHttpResult>> Rename(IMediator mediator, [FromBody] RenameResource renameResource, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameUser(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, UserView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteUsersInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
