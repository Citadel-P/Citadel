using System.Security.Claims;
using Application.Permissions.Requirements;
using Domain.Entities;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;

namespace Application.Permissions;

public interface IContainerPermissionService
{
    Task<PermissionsMetadata> GetContainerPermissions(Container container);
}

internal class ContainerPermissionService(IAuthorizationService authorizationService, IHttpContextAccessor httpContextAccessor) : IContainerPermissionService
{
    public async Task<PermissionsMetadata> GetContainerPermissions(Container container)
    {
        var user = httpContextAccessor.HttpContext?.User 
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var canEdit = (await authorizationService.AuthorizeAsync(user, container, new EditContainerRequirement())).Succeeded;
        var canDelete = (await authorizationService.AuthorizeAsync(user, container, new DeleteContainerRequirement())).Succeeded;

        return new PermissionsMetadata(CanEdit: canEdit, CanDelete: canDelete);
    }
}
