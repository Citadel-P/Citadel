using Application.Permissions;
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
    private static readonly TimeSpan ActorScopeCacheTtl = TimeSpan.FromMinutes(30);
    private static readonly TimeSpan PermissionCacheTtl = TimeSpan.FromMinutes(5);

    // Very small, request-local dedupe store.
    private readonly Dictionary<PermissionCacheKey, PermissionMetadata> requestCache = [];

    public Task<Result> EnforceAsync<TMessage>(TMessage message, ClaimsPrincipal user, CancellationToken cancellationToken = default) where TMessage : notnull
        => PermissionPipeline.Enforce(
            message,
            user.GetUserId(),
            this,
            cancellationToken
        );

    public async Task<PermissionMetadata> ResolvePermissionsAsync(Guid userId, ResourceType resourceType, Guid? resourceId, CancellationToken ct = default)
    {
        if (userId == Guid.Empty)
            return default!;

        var key = new PermissionCacheKey(userId, resourceType, resourceId);

        if (requestCache.TryGetValue(key, out var requestCached))
            return requestCached;

        var permCacheKey = $"perm:{userId}:{(int)resourceType}:{resourceId?.ToString() ?? string.Empty}";

        if (memoryCache.TryGetValue<PermissionMetadata>(permCacheKey, out var memCached))
        {
            requestCache[key] = memCached;
            return memCached;
        }

        var actorIds = await GetActorScopeAsync(userId, ct);

        var permissions = await uow.Users.GetEffectivePermissionsAsync(actorIds, resourceType, resourceId, ct);

        var cacheOptions = new MemoryCacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = PermissionCacheTtl
        };

        memoryCache.Set(permCacheKey, permissions, cacheOptions);

        // Maintain an index of permission cache keys per user so evictors can remove them when actor scope changes.
        // Index must outlive the permission entries it tracks to avoid orphaned perm keys.
        var permIndexKey = $"perm-index:{userId}";
        var permIndexOptions = new MemoryCacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1)
        };

        if (memoryCache.TryGetValue<HashSet<string>>(permIndexKey, out var existingIndex) && existingIndex is not null)
        {
            var updated = new HashSet<string>(existingIndex, StringComparer.Ordinal) { permCacheKey };
            memoryCache.Set(permIndexKey, updated, permIndexOptions);
        }
        else
        {
            memoryCache.Set(permIndexKey, new HashSet<string> { permCacheKey }, permIndexOptions);
        }
        requestCache[key] = permissions;

        return permissions;
    }

    private async ValueTask<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken ct)
    {
        var actorCacheKey = $"actor-scope:{userId}";

        if (memoryCache.TryGetValue<Guid[]>(actorCacheKey, out var cachedActorIds))
        {
            if (cachedActorIds != null && cachedActorIds.Length > 0)
                return cachedActorIds;
        }

        var actorIds = await uow.Users.GetActorScopeAsync(userId, ct);

        var cacheEntryOptions = new MemoryCacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = ActorScopeCacheTtl
        };

        memoryCache.Set(actorCacheKey, actorIds, cacheEntryOptions);
        return actorIds;
    }
}
