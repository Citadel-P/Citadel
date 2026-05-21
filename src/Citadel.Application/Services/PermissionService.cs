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
            return PermissionMetadata.Empty;

        var key = new PermissionCacheKey(userId, resourceType, resourceId);

        if (requestCache.TryGetValue(key, out var requestCached))
            return requestCached;

        if (memoryCache.TryGetValue<PermissionMetadata>(key, out var memCached))
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

        memoryCache.Set(key, permissions, cacheOptions);

        // Maintain an index of permission cache keys per user so evictors can remove them when actor scope changes.
        // Index must outlive the permission entries it tracks to avoid orphaned perm keys.
        var permIndexKey = $"perm-index:{userId}";
        var permIndexOptions = new MemoryCacheEntryOptions
        {
            AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1)
        };

        if (memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(permIndexKey, out var existingIndex) && existingIndex is not null)
        {
            var updated = new HashSet<PermissionCacheKey>(existingIndex) { key };
            memoryCache.Set(permIndexKey, updated, permIndexOptions);
        }
        else
        {
            memoryCache.Set(permIndexKey, new HashSet<PermissionCacheKey> { key }, permIndexOptions);
        }
        requestCache[key] = permissions;

        return permissions;
    }

    public async Task<IReadOnlyDictionary<Guid, PermissionMetadata>> ResolvePermissionsAsyncForIds(Guid userId, ResourceType resourceType, Guid[] resourceIds, CancellationToken ct = default)
    {
        if (userId == Guid.Empty)
            return resourceIds?.ToDictionary(id => id, id => PermissionMetadata.Empty) ?? [];

        // Short-circuit empty
        if (resourceIds == null || resourceIds.Length == 0)
            return new Dictionary<Guid, PermissionMetadata>();

        HashSet<Guid> uniqueIds = [];
        foreach (var id in resourceIds)
            uniqueIds.Add(id);
        var result = new Dictionary<Guid, PermissionMetadata>(uniqueIds.Count);

        var toFetch = new List<Guid>(uniqueIds.Count);

        // First pass: request-local and memory cache
        foreach (var rid in uniqueIds)
        {
            var key = new PermissionCacheKey(userId, resourceType, rid);
            if (requestCache.TryGetValue(key, out var reqCached))
            {
                result[rid] = reqCached;
                continue;
            }

            if (memoryCache.TryGetValue<PermissionMetadata>(key, out var memCached))
            {
                requestCache[key] = memCached;
                result[rid] = memCached;
                continue;
            }

            toFetch.Add(rid);
        }

        if (toFetch.Count == 0)
            return result;

        // Fetch actor scope once
        var actorIds = await GetActorScopeAsync(userId, ct);

        // Fetch all missing permissions
        var toFetchArray = toFetch.ToArray();
        var fetched = await uow.Users.GetEffectivePermissionsBatchAsync(actorIds, resourceType, toFetchArray, ct);

        var cacheOptions = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl };
        var permIndexKey = $"perm-index:{userId}";
        var permIndexOptions = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1) };

        // Gather or create a single index set and update it once at the end to avoid cloning per item
        var existingIndex = memoryCache.Get<HashSet<PermissionCacheKey>>(permIndexKey);
        var indexSet = existingIndex is not null ? [.. existingIndex] : new HashSet<PermissionCacheKey>();

        foreach (var rid in toFetchArray)
        {
            if (!fetched.TryGetValue(rid, out var permissions))
                permissions = PermissionMetadata.Empty;

            var cacheKey = new PermissionCacheKey(userId, resourceType, rid);
            memoryCache.Set(cacheKey, permissions, cacheOptions);
            requestCache[cacheKey] = permissions;
            result[rid] = permissions;

            indexSet.Add(cacheKey);
        }

        // Persist permission index once
        memoryCache.Set(permIndexKey, indexSet, permIndexOptions);

        return result;
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
