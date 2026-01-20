using System.Diagnostics.CodeAnalysis;
using Domain.Contracts.Resources;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// A centralized, thread-safe cache for managing the mapping of platform containers (online platforms only).
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
    bool TryGetCacheEntry(Guid platformId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry, [MaybeNullWhen(true)] out Error error);

    /// <summary>
    /// Tries to get all cache entries.
    /// </summary>
    bool TryGetCacheEntries([MaybeNullWhen(false)] out IEnumerable<PlatformCacheEntry> cacheEntries, [MaybeNullWhen(true)] out Error error);
    /// <summary>
    /// Tries to get the cache entry for a specific platform by its container ID.
    /// </summary>
    bool TryGetPlatformWithContainer(string containerId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry);

    /// <summary>
    /// Tries to get platforms cache entries by a list of container IDs.
    /// </summary>
    bool TryGetPlatformsWithContainers(string[] containersId, [MaybeNullWhen(false)] out List<PlatformCacheEntry> cacheEntries);
}
