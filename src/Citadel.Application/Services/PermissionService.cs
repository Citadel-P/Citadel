using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Microsoft.Extensions.Caching.Memory;
using System.Security.Claims;

namespace Application.Services;

internal class PermissionService(IUnitOfWork uow, IMemoryCache memoryCache) : IPermissionService
{
    private static readonly TimeSpan PermissionCacheTtl = TimeSpan.FromSeconds(60);

    public Task<Result> EnforceAsync<TMessage>(TMessage message, ClaimsPrincipal user, CancellationToken cancellationToken = default) where TMessage : notnull
        => PermissionPipeline.Enforce(
            message,
            user.GetUserId(),
            this,
            cancellationToken
        );

    public Task<bool> HasPermissionAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid? resourceId,
        CancellationToken ct)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermission == SpecificPermission.None ? null : [specificPermission]))
            throw new InvalidOperationException(
                $"Invalid runtime permission check: [{resourceType}]-[{permissionLevel}] with specific [{specificPermission}] is not an allowed combination.");

        // Per resource permissions are not cached, as they are expected to be less common and more dynamic.
        if (resourceId is not null)
        {
            return uow.Users.HasPermissionAsync(userId, resourceType, permissionLevel, specificPermission, resourceId, ct);
        }

        var cacheKey = new PermissionCacheKey(userId, resourceType, permissionLevel, specificPermission);
        return memoryCache.GetOrCreateAsync(cacheKey, async entry =>
        {
            entry.SlidingExpiration = PermissionCacheTtl;
            return await uow.Users.HasPermissionAsync(userId, resourceType, permissionLevel, specificPermission, null, ct);
        });
    }
    public Task<bool> HasPermissionForAllAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid[] resourceIds,
        CancellationToken ct)
    {
        if (!PermissionMatrix.IsAllowed(resourceType, permissionLevel, specificPermission == SpecificPermission.None ? null : [specificPermission]))
            throw new InvalidOperationException(
                $"Invalid runtime permission check: [{resourceType}]-[{permissionLevel}] with specific [{specificPermission}] is not an allowed combination.");

        if (resourceIds.Length == 0)
            return Task.FromResult(true);

        return uow.Users.HasPermissionForAllAsync(userId, resourceType, permissionLevel, specificPermission, resourceIds, ct);
    }

    private readonly record struct PermissionCacheKey(Guid UserId, ResourceType ResourceType, PermissionLevel PermissionLevel, SpecificPermission SpecificPermission);
}
