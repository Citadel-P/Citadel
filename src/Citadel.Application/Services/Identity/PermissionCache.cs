using Application.Permissions;
using Hosting.Common.Attributes;
using Microsoft.Extensions.Caching.Memory;

namespace Application.Services.Identity;

internal sealed class PermissionCache(IMemoryCache memoryCache) : IPermissionCache
{
    private static readonly TimeSpan PermissionCacheTtl = TimeSpan.FromMinutes(5);

    public PermissionMetadata? Get(PermissionCacheKey key)
    {
        if (memoryCache.TryGetValue<PermissionMetadata>(key, out var value))
            return value;

        return null;
    }

    public void Set(PermissionCacheKey key, PermissionMetadata meta)
    {
        var options = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl };
        memoryCache.Set(key, meta, options);
        AddToIndex(key.UserId, key);
    }

    public IReadOnlyCollection<PermissionCacheKey> GetIndex(Guid userId)
    {
        var idx = memoryCache.Get<HashSet<PermissionCacheKey>>(Hosting.Common.Constants.CacheKeys.PermIndex(userId));
        return idx is not null ? idx : Array.Empty<PermissionCacheKey>();
    }

    public void AddToIndex(Guid userId, PermissionCacheKey key)
    {
        var permIndexKey = Hosting.Common.Constants.CacheKeys.PermIndex(userId);
        var options = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1) };

        if (memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(permIndexKey, out var existing) && existing is not null)
        {
            var updated = new HashSet<PermissionCacheKey>(existing) { key };
            memoryCache.Set(permIndexKey, updated, options);
        }
        else
        {
            memoryCache.Set(permIndexKey, new HashSet<PermissionCacheKey> { key }, options);
        }
    }

    public void AddManyToIndex(Guid userId, IEnumerable<PermissionCacheKey> keys)
    {
        var permIndexKey = Hosting.Common.Constants.CacheKeys.PermIndex(userId);
        var options = new MemoryCacheEntryOptions { AbsoluteExpirationRelativeToNow = PermissionCacheTtl + TimeSpan.FromMinutes(1) };

        var existing = memoryCache.Get<HashSet<PermissionCacheKey>>(permIndexKey);
        var indexSet = existing is not null ? [.. existing] : new HashSet<PermissionCacheKey>();

        foreach (var k in keys)
            indexSet.Add(k);

        memoryCache.Set(permIndexKey, indexSet, options);
    }

    public void Remove(PermissionCacheKey key)
    {
        memoryCache.Remove(key);
    }

    public void InvalidateUser(Guid userId)
    {
        var permIndexKey = Hosting.Common.Constants.CacheKeys.PermIndex(userId);
        if (memoryCache.TryGetValue<HashSet<PermissionCacheKey>>(permIndexKey, out var permissionKeys) && permissionKeys is not null)
        {
            foreach (var key in permissionKeys)
            {
                memoryCache.Remove(key);
            }
        }

        memoryCache.Remove(permIndexKey);
    }
}
