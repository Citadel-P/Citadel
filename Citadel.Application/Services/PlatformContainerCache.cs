using System.Collections.Concurrent;
using System.Diagnostics.CodeAnalysis;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;

namespace Application.Services;

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