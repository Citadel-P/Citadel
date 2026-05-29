using Application.Permissions;
using Citadel.SourceGen;
using Domain.Contracts.Interfaces;
using Application.Services.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;

namespace Application.Services;

internal class PermissionService(IUnitOfWork uow, IPermissionCache permissionCache, IActorScopeProvider actorScopeProvider) : IPermissionService
{
    // Request-local dedupe store.
    private readonly Dictionary<PermissionCacheKey, PermissionMetadata> requestCache = [];

    public Task<Result> EnforceAsync<TMessage>(TMessage message, Guid userId, CancellationToken cancellationToken = default) where TMessage : notnull
        => PermissionPipeline.Enforce(
            message,
            userId,
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

        if (permissionCache.Get(key) is PermissionMetadata memCached)
        {
            requestCache[key] = memCached;
            return memCached;
        }

        var actorIds = await actorScopeProvider.GetActorScopeAsync(userId, ct);

        var permissions = await uow.Users.GetEffectivePermissionsAsync(actorIds, resourceType, resourceId, ct);

        permissionCache.Set(key, permissions);
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

            if (permissionCache.Get(key) is PermissionMetadata memCached)
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
        var actorIds = await actorScopeProvider.GetActorScopeAsync(userId, ct);

        // Fetch all missing permissions
        var toFetchArray = toFetch.ToArray();
        var fetched = await uow.Users.GetEffectivePermissionsBatchAsync(actorIds, resourceType, toFetchArray, ct);

        var indexSet = new List<PermissionCacheKey>();

        foreach (var rid in toFetchArray)
        {
            if (!fetched.TryGetValue(rid, out var permissions))
                permissions = PermissionMetadata.Empty;

            var cacheKey = new PermissionCacheKey(userId, resourceType, rid);
            permissionCache.Set(cacheKey, permissions);
            requestCache[cacheKey] = permissions;
            result[rid] = permissions;

            indexSet.Add(cacheKey);
        }
        // Persist permission index once
        permissionCache.AddManyToIndex(userId, indexSet);

        return result;
    }

}
