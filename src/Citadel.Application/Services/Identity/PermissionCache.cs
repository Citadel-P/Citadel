using Application.Permissions;
using Hosting.Common.Attributes;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal sealed class PermissionCache(IMemoryCache memoryCache) : IPermissionCache
{
    private static readonly TimeSpan PermissionCacheTtl = TimeSpan.FromMinutes(5);
    private readonly object sync = new();

    public PermissionMetadata? Get(PermissionCacheKey key)
    {
        if (memoryCache.TryGetValue<PermissionMetadata>(key, out var value))
            return value;

        return null;
    }

    public void Set(PermissionCacheKey key, PermissionMetadata meta)
    {
        lock (sync)
        {
            var options = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl };
            memoryCache.Set(key, meta, options);

            var indexKey = Hosting.Common.Constants.CacheKeys.PermIndex(key.UserId);
            if (!memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(indexKey, out var keys) || keys is null)
            {
                keys = [];
                memoryCache.Set(
                    indexKey,
                    keys,
                    new MemoryCacheEntryOptions
                    {
                        AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1)
                    });
            }

            keys.Add(key);
        }
    }

    public IReadOnlyCollection<PermissionCacheKey> GetIndex(Guid userId)
    {
        lock (sync)
        {
            var indexKey = Hosting.Common.Constants.CacheKeys.PermIndex(userId);
            return memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(indexKey, out var keys) && keys is not null
                ? [.. keys]
                : [];
        }
    }

    public void Remove(PermissionCacheKey key)
    {
        lock (sync)
        {
            memoryCache.Remove(key);
            var indexKey = Hosting.Common.Constants.CacheKeys.PermIndex(key.UserId);
            if (memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(indexKey, out var keys) && keys is not null)
                keys.Remove(key);
        }
    }

    public void InvalidateUser(Guid userId)
    {
        lock (sync)
        {
            var indexKey = Hosting.Common.Constants.CacheKeys.PermIndex(userId);
            if (memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(indexKey, out var keys) && keys is not null)
            {
                foreach (var key in keys)
                    memoryCache.Remove(key);
            }

            memoryCache.Remove(indexKey);
        }
    }
}
