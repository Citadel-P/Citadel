using System.Collections.Concurrent;
using System.Diagnostics.CodeAnalysis;

namespace Infrastructure.Services;

/// <summary>
/// A centralized, thread-safe cache for managing the mapping of platform containers.
/// Maps Platform ID -> (Container Runtime ID -> Container Database ID).
/// </summary>
public interface IPlatformContainerCache
{
    /// <summary>
    /// Replaces all container data for a specific platform. Used for full sync operations.
    /// </summary>
    void ReplacePlatformContainers(Guid platformId, IReadOnlyDictionary<string, Guid> containers);

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
}
internal class PlatformContainerCache : IPlatformContainerCache
{
    private readonly ConcurrentDictionary<Guid, Dictionary<string, Guid>> cache = [];

    /// <inheritdoc />
    public void ReplacePlatformContainers(Guid platformId, IReadOnlyDictionary<string, Guid> containers)
    {
        cache[platformId] = new Dictionary<string, Guid>(containers);
    }

    /// <inheritdoc />
    public bool TryAddContainer(Guid platformId, string containerId, Guid dbId)
    {
        if (cache.TryGetValue(platformId, out var platformContainers))
        {
            platformContainers[containerId] = dbId;
            return true;
        }
        return false; // Platform not yet in cache, full sync will add it.
    }

    /// <inheritdoc />
    public bool TryRemoveContainer(Guid platformId, string containerId)
    {
        if (cache.TryGetValue(platformId, out var platformContainers))
        {
            return platformContainers.Remove(containerId, out _);
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
            containers = inner;
            return true;
        }
        return false;
    }
}
