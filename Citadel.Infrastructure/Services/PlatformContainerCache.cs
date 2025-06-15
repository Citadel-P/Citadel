using System.Collections.Concurrent;
using System.Diagnostics.CodeAnalysis;
using Domain.Contracts.Resources;

namespace Infrastructure.Services;

/// <summary>
/// A centralized, thread-safe cache for managing the mapping of platform containers.
/// </summary>
public interface IPlatformContainerCache
{
    /// <summary>
    /// Replaces cache entry for a specific platform.
    /// </summary>
    void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry);

    /// <summary>
    /// Adds a single container mapping to the cache for a specific platform.
    /// </summary>
    bool TryAddContainer(Guid platformId, string containerId, Guid dbId);

    /// <summary>
    /// Removes a single container mapping from the cache.
    /// </summary>
    bool TryRemoveContainer(Guid platformId, string containerId);

    /// <summary>
    /// Removes all container data for a specific platform, e.g., when it goes offline.
    /// </summary>
    bool EvictPlatform(Guid platformId);

    /// <summary>
    /// Tries to get the container list for a specific platform.
    /// </summary>
    bool TryGetContainers(Guid platformId, [MaybeNullWhen(false)] out IReadOnlyDictionary<string, Guid> containers);

    /// <summary>
    /// Tries to get the cache entry for a specific platform.
    /// </summary>
    bool TryGetCacheEntry(Guid platformId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry);
}
internal class PlatformContainerCache : IPlatformContainerCache
{
    private readonly ConcurrentDictionary<Guid, PlatformCacheEntry> cache = [];

    /// <inheritdoc />
    public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry)
    {
        cache[platformId] = cacheEntry;
    }

    /// <inheritdoc />
    public bool TryAddContainer(Guid platformId, string containerId, Guid dbId)
    {
        if (cache.TryGetValue(platformId, out var entry))
        {
            entry.Containers[containerId] = dbId;
            return true;
        }
        return false; // Platform not yet in cache, full sync will add it.
    }

    /// <inheritdoc />
    public bool TryRemoveContainer(Guid platformId, string containerId)
    {
        if (cache.TryGetValue(platformId, out var platformContainers))
        {
            return platformContainers.Containers.Remove(containerId, out _);
        }
        return false;
    }

    /// <inheritdoc />
    public bool EvictPlatform(Guid platformId) => cache.TryRemove(platformId, out _);

    /// <inheritdoc />
    public bool TryGetContainers(Guid platformId, [MaybeNullWhen(false)] out IReadOnlyDictionary<string, Guid> containers)
    {
        containers = null;
        if (cache.TryGetValue(platformId, out var inner))
        {
            containers = inner.Containers;
            return true;
        }
        return false;
    }

    /// <inheritdoc />
    public bool TryGetCacheEntry(Guid platformId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry)
    {
        cacheEntry = null;
        if (cache.TryGetValue(platformId, out var inner))
        {
            cacheEntry = inner;
            return true;
        }
        return false;
    }
}