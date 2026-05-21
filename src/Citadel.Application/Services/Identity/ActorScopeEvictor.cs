using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal interface IActorScopeEvictor
{
    Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default);
    void EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default);
}

internal sealed class ActorScopeEvictor(IUnitOfWork uow, IMemoryCache memoryCache) : IActorScopeEvictor
{
    public async Task EvictPermissionsForActorAsync(Guid actorId, CancellationToken cancellationToken = default)
    {
        var userIds = await uow.Teams.GetUserIdsByActorIdAsync(actorId, cancellationToken);

        foreach (var id in (userIds ?? []).Distinct())
        {
            var actorCacheKey = $"actor-scope:{id}";
            memoryCache.Remove(actorCacheKey);

            // Also evict any permission cache entries associated with this user
            var permIndexKey = $"perm-index:{id}";
            // Prefer typed PermissionCacheKey index if present
            if (memoryCache.TryGetValue<HashSet<Permissions.PermissionCacheKey>>(permIndexKey, out var typedPermKeys) && typedPermKeys is not null)
            {
                foreach (var pk in typedPermKeys)
                {
                    memoryCache.Remove(pk);
                }

                memoryCache.Remove(permIndexKey);
            }
            else if (memoryCache.TryGetValue<HashSet<string>>(permIndexKey, out var permKeys) && permKeys is not null)
            {
                foreach (var pk in permKeys)
                {
                    memoryCache.Remove(pk);
                }

                memoryCache.Remove(permIndexKey);
            }
        }
    }

    public void EvictUsers(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default)
    {
        foreach (var id in userIds ?? [])
        {
            var actorCacheKey = $"actor-scope:{id}";
            memoryCache.Remove(actorCacheKey);

            var permIndexKey = $"perm-index:{id}";
            if (memoryCache.TryGetValue<HashSet<global::Application.Permissions.PermissionCacheKey>>(permIndexKey, out var typedPermKeys) && typedPermKeys is not null)
            {
                foreach (var pk in typedPermKeys)
                {
                    memoryCache.Remove(pk);
                }

                memoryCache.Remove(permIndexKey);
            }
            else if (memoryCache.TryGetValue<HashSet<string>>(permIndexKey, out var permKeys) && permKeys is not null)
            {
                foreach (var pk in permKeys)
                {
                    memoryCache.Remove(pk);
                }

                memoryCache.Remove(permIndexKey);
            }
        }
    }
}
