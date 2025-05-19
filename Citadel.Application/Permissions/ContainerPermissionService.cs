using System.Security.Claims;
using Application.Permissions.Requirements;
using Infrastructure.Entities;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Http;

namespace Application.Permissions;

public interface IContainerPermissionService
{
    Task<PermissionsMetadata> GetContainerPermissions(ContainerInfo containerInfo);
}

internal class ContainerPermissionService(IAuthorizationService authorizationService, IHttpContextAccessor httpContextAccessor) : IContainerPermissionService
{
    public async Task<PermissionsMetadata> GetContainerPermissions(ContainerInfo containerInfo)
    {
        var user = httpContextAccessor.HttpContext?.User 
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var canEdit = (await authorizationService.AuthorizeAsync(user, containerInfo, new EditContainerRequirement())).Succeeded;
        var canDelete = (await authorizationService.AuthorizeAsync(user, containerInfo, new DeleteContainerRequirement())).Succeeded;

        return new PermissionsMetadata(CanEdit: canEdit, CanDelete: canDelete);
    }
}
