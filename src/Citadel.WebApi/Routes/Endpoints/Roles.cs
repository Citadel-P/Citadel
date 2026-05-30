using Application.Features.Identity.Roles.Commands;
using Application.Features.Identity.Roles.Queries;
using Application.Permissions;
using Domain.Contracts.Resources.Role;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Identity.Roles;

namespace WebApi.Routes.Endpoints;

public static class Roles
{
    public static async Task<Results<Ok<RoleView>, ProblemHttpResult>> Create(IMediator mediator, [FromBody] RoleInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResult(result, RoleView.Map);
    }

    public static async Task<Results<Ok<RolesView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetAllRoles(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, RolesView.Map);
    }

    public static async Task<Results<Ok<RoleView>, ProblemHttpResult>> Get(IMediator mediator, [Description("Role id")] Guid id, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new GetRole(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, RoleView.Map);
    }

    public static async Task<Results<Ok<RoleView>, ProblemHttpResult>> PatchPermissions(
        IMediator mediator, 
        [FromRoute][Description("Role ID")] Guid id, 
        PatchRolePermissionsInputPatchDocument patchInput, 
        CancellationToken cancellationToken)
    {
        JsonMergePatchDocument<PatchRolePermissionsModel> mapped = patchInput.Map<PatchRolePermissionsInput, PatchRolePermissionsModel>();
        var result = await mediator.Send(new PatchRolePermissions(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, RoleView.Map);
    }

    public static async Task<Results<Ok<RoleView>, ProblemHttpResult>> Rename(IMediator mediator, [FromBody] RenameResource renameResource, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenameRole(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, RoleView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteRolesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
