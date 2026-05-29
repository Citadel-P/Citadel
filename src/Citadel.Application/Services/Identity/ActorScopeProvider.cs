using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal sealed class ActorScopeProvider(IUnitOfWork uow, IMemoryCache memoryCache) : IActorScopeProvider
{
    private static readonly TimeSpan ActorScopeCacheTtl = TimeSpan.FromMinutes(30);

    private static string Key(Guid userId) => Constants.CacheKeys.ActorScope(userId);

    public async Task<Guid[]> GetActorScopeAsync(Guid userId, CancellationToken cancellationToken = default)
    {
        if (userId == Guid.Empty)
            return [];

        if (memoryCache.TryGetValue<Guid[]>(Key(userId), out var cached) && cached is not null && cached.Length > 0)
            return cached;

        var actorIds = await uow.Users.GetActorScopeAsync(userId, cancellationToken) ?? [];

        var opts = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = ActorScopeCacheTtl };
        memoryCache.Set(Key(userId), actorIds, opts);
        return actorIds;
    }

    public bool TryGetCachedActorScope(Guid userId, out Guid[]? actorIds)
    {
        if (memoryCache.TryGetValue<Guid[]>(Key(userId), out var cached) && cached is not null && cached.Length > 0)
        {
            actorIds = cached;
            return true;
        }

        actorIds = [];
        return false;
    }

    public Task InvalidateAsync(Guid userId, CancellationToken cancellationToken = default)
    {
        memoryCache.Remove(Key(userId));
        return Task.CompletedTask;
    }

    public Task InvalidateManyAsync(IEnumerable<Guid> userIds, CancellationToken cancellationToken = default)
    {
        foreach (var id in userIds ?? [])
            memoryCache.Remove(Key(id));

        return Task.CompletedTask;
    }
}
