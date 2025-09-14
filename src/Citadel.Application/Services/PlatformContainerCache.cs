using System.Collections.Concurrent;
using System.Diagnostics.CodeAnalysis;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Hosting.Common.ErrorTypes;
using LightResults;

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
        return false;
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
    public bool TryGetCacheEntry(Guid platformId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry, [MaybeNullWhen(true)] out Error error)
    {
        cacheEntry = null;
        error = null;
        if (cache.TryGetValue(platformId, out var inner))
        {
            cacheEntry = inner;
            return true;
        }
        error = new NotFoundError("Platform is disconnected or unavailable.");
        return false;
    }

    /// <inheritdoc />
    public bool TryGetCacheEntries([MaybeNullWhen(false)] out IEnumerable<PlatformCacheEntry> cacheEntries, [MaybeNullWhen(true)] out Error error)
    {
        cacheEntries = null;
        error = null;
        
        if (cache.Keys.Count != 0)
        {
            cacheEntries = cache.Values.AsEnumerable();
            return true;
        }
        error = new NotFoundError("No platform is currently connected.");
        return false;
    }

    /// <inheritdoc />
    public bool TryGetPlatformByContainerId(string containerId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry)
    {
        cacheEntry = null;
        foreach (var (_, platformCacheEntry) in cache)
        {
            foreach (var key in platformCacheEntry.Containers.Keys)
            {
                if (key.StartsWith(containerId, StringComparison.OrdinalIgnoreCase))
                {
                    cacheEntry = platformCacheEntry;
                    return true;
                }
            }
        }

        return false;
    }

    /// <inheritdoc />
    public bool TryGetPlatformsByContainersId(string[] containersId, [MaybeNullWhen(false)] out List<PlatformCacheEntry> cacheEntries)
    {
        cacheEntries = null;
        var foundEntries = new List<PlatformCacheEntry>();
        foreach (var (_, platformCacheEntry) in cache)
        {
            foreach (var key in platformCacheEntry.Containers.Keys)
            {
                var found = false;
                PlatformCacheEntry? cacheEntry = null;
                foreach (var containerId in containersId)
                {
                    if (key.StartsWith(containerId, StringComparison.OrdinalIgnoreCase))
                    {
                        
                        if (!found)
                        {
                            cacheEntry = new PlatformCacheEntry(
                             Address: platformCacheEntry.Address,
                             ConnectorType: platformCacheEntry.ConnectorType,
                             Containers: []);
                        }
                        if (cacheEntry != null)
                        {
                            cacheEntry.Containers[key] = platformCacheEntry.Containers[key];    
                        }
                        found = true;

                        foundEntries.Add(platformCacheEntry);
                    }
                }

                if (found && cacheEntry != null)
                {
                    cacheEntries ??= [];
                    cacheEntries.Add(cacheEntry);
                }

            }
        }
        return foundEntries.Count > 0;
    }
}