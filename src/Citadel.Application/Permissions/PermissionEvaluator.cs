using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.AspNetCore.Http;
using System.Collections.Generic;
using System.Linq;
using System.Threading;

namespace Application.Permissions;

public interface IPermissionEvaluator
{
    Task<PermissionMetadata> EvaluateAsync(Guid resourceId, ResourceType resourceType, CancellationToken ct = default);
    Task<IReadOnlyDictionary<Guid, PermissionMetadata>> EvaluateAsync(Guid[] resourceIds, ResourceType resourceType, CancellationToken ct = default);
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

    public async Task<IReadOnlyDictionary<Guid, PermissionMetadata>> EvaluateAsync(Guid[] resourceIds, ResourceType resourceType, CancellationToken ct = default)
    {
        var user = (contextAccessor.HttpContext?.User);
        if (user is null || user.Identity?.IsAuthenticated != true)
        {
            return new Dictionary<Guid, PermissionMetadata>();
        }

        if (user.IsAdmin())
        {
            return resourceIds.ToDictionary(id => id, id => Helpers.AdminPermissions);
        }

        var userId = user.GetUserId();

        if (userId == Guid.Empty)
        {
            return new Dictionary<Guid, PermissionMetadata>();
        }

        var permissions = await permissionService.ResolvePermissionsAsyncForIds(userId, resourceType, resourceIds, ct);
        return permissions;
    }
}
