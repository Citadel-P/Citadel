using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.AspNetCore.Http;

namespace Application.Permissions;

public interface IPermissionEvaluator
{
    Task<PermissionMetadata> EvaluateAsync(Guid resourceId, ResourceType resourceType, CancellationToken ct = default);
}

internal sealed class PermissionEvaluator(IPermissionService permissionService, IHttpContextAccessor contextAccessor) : IPermissionEvaluator
{
    public async Task<PermissionMetadata> EvaluateAsync(Guid resourceId, ResourceType resourceType, CancellationToken ct = default)
    {
        var user = (contextAccessor.HttpContext?.User);
        if (user is null || user.Identity?.IsAuthenticated != true)
        {
            return default!;
        }

        if (user.IsAdmin())
        {
            return Helpers.AdminPermissions;
        }

        var userId = user.GetUserId();

        if (userId == Guid.Empty)
        {
            return default!;
        }

        var permissions = await permissionService.ResolvePermissionsAsync(userId, resourceType, resourceId, ct);
        return permissions;
    }
}
