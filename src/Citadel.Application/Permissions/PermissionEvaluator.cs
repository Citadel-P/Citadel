using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.Pipelines.Interfaces;

namespace Application.Permissions;

public interface IPermissionEvaluator
{
    Task<PermissionMetadata> EvaluateAsync(ResourceType resourceType, CancellationToken ct = default);
    Task<PermissionMetadata> EvaluateAsync(Guid resourceId, ResourceType resourceType, CancellationToken ct = default);
    Task<IReadOnlyDictionary<Guid, PermissionMetadata>> EvaluateAsync(Guid[] resourceIds, ResourceType resourceType, CancellationToken ct = default);
}

internal sealed class PermissionEvaluator(IPermissionService permissionService, IUserContextAccessor userContextAccessor) : IPermissionEvaluator
{
    public async Task<PermissionMetadata> EvaluateAsync(ResourceType resourceType, CancellationToken ct = default)
    {
        var user = userContextAccessor.Current;
        if (user is null || !user.IsAuthenticated)

        {
            return default!;
        }

        var userId = user.UserId;

        if (userId == Guid.Empty)
        {
            return default!;
        }

        if (user.IsAdmin)
        {
            return Helpers.AdminPermissions;
        }

        var permissions = await permissionService.ResolvePermissionsAsync(userId, resourceType, null, ct);
        return permissions;
    }

    public async Task<PermissionMetadata> EvaluateAsync(Guid resourceId, ResourceType resourceType, CancellationToken ct = default)
    {
        var user = userContextAccessor.Current;
        if (user is null || !user.IsAuthenticated)

        {
            return default!;
        }

        var userId = user.UserId;

        if (userId == Guid.Empty)
        {
            return default!;
        }

        if (user.IsAdmin)
        {
            return Helpers.AdminPermissions;
        }

        var permissions = await permissionService.ResolvePermissionsAsync(userId, resourceType, resourceId, ct);
        return permissions;
    }

    public async Task<IReadOnlyDictionary<Guid, PermissionMetadata>> EvaluateAsync(Guid[] resourceIds, ResourceType resourceType, CancellationToken ct = default)
    {
        var user = userContextAccessor.Current;
        if (user is null || !user.IsAuthenticated)
        {
            return new Dictionary<Guid, PermissionMetadata>();
        }

        var userId = user.UserId;

        if (userId == Guid.Empty)
        {
            return new Dictionary<Guid, PermissionMetadata>();
        }

        if (user.IsAdmin)
        {
            return resourceIds.ToDictionary(id => id, id => Helpers.AdminPermissions);
        }

        var permissions = await permissionService.ResolvePermissionsAsyncForIds(userId, resourceType, resourceIds, ct);
        return permissions;
    }
}
