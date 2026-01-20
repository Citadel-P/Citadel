using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Hosting.Common.ErrorTypes;
using LightResults;
using System.Collections.Immutable;
using System.Diagnostics.CodeAnalysis;

namespace Application.Services;

internal sealed class PlatformContainerCache : IPlatformContainerCache
{
    // Atomic snapshot
    private volatile PlatformSnapshot _snapshot =new([], []);

    private static string NormalizeId(string id) => id.ToLowerInvariant().Trim();
    private const int ShortIdLength = 12;

    public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry entry)
    {
        while (true)
        {
            var oldSnap = _snapshot;
            var mapBuilder = oldSnap.ContainerToPlatform.ToBuilder();

            // Remove old containers for this platform
            if (oldSnap.Platforms.TryGetValue(platformId, out var oldPlatform))
            {
                foreach (var cid in oldPlatform.Containers.Keys)
                    mapBuilder.Remove(cid);
            }

            // Add new containers with full 64-char IDs
            var newContainers = entry.Containers
                .ToImmutableDictionary(
                    kvp => NormalizeId(kvp.Key),
                    kvp => kvp.Value);

            foreach (var cid in newContainers.Keys)
                mapBuilder[cid] = platformId;

            var view = new PlatformView(
                entry.Id, 
                entry.Address, 
                entry.ConnectorType, 
                newContainers);

            var newSnap = oldSnap with
            {
                Platforms = oldSnap.Platforms.SetItem(platformId, view),
                ContainerToPlatform = mapBuilder.ToImmutable()
            };

            if (ReferenceEquals(
                    Interlocked.CompareExchange(ref _snapshot, newSnap, oldSnap),
                    oldSnap))
                return;
        }
    }

    public bool TryAddContainer(Guid platformId, string containerId, Guid dbId)
    {
        var nid = NormalizeId(containerId);

        while (true)
        {
            var oldSnap = _snapshot;

            if (!oldSnap.Platforms.TryGetValue(platformId, out var platform)) 
                return false;

            var newPlatform = platform with 
            {
                Containers = platform.Containers.SetItem(nid, dbId) 
            };

            var newSnap = oldSnap with
            {
                Platforms = oldSnap.Platforms.SetItem(platformId, newPlatform),
                ContainerToPlatform = oldSnap.ContainerToPlatform.SetItem(nid, platformId)
            };

            if (ReferenceEquals(
                    Interlocked.CompareExchange(ref _snapshot, newSnap, oldSnap), 
                    oldSnap))
                return true;
        }
    }

    public bool TryRemoveContainer(Guid platformId, string containerId)
    {
        while (true)
        {
            var oldSnap = _snapshot;
            var fullId = ResolveFullId(oldSnap, containerId);
            if (fullId == null) return false;

            if (!oldSnap.Platforms.TryGetValue(platformId, out var platform)) return false;
            if (!platform.Containers.ContainsKey(fullId)) return false;

            var newPlatform = platform with 
            { 
                Containers = platform.Containers.Remove(fullId) 
            };

            var newSnap = oldSnap with
            {
                Platforms = oldSnap.Platforms.SetItem(platformId, newPlatform),
                ContainerToPlatform = oldSnap.ContainerToPlatform.Remove(fullId)
            };

            if (ReferenceEquals(
                    Interlocked.CompareExchange(ref _snapshot, newSnap, oldSnap),
                    oldSnap))
                return true;
        }
    }

    public bool EvictPlatform(Guid platformId)
    {
        while (true)
        {
            var oldSnap = _snapshot;
            if (!oldSnap.Platforms.TryGetValue(platformId, out var platform)) return false;

            var mapBuilder = oldSnap.ContainerToPlatform.ToBuilder();
            foreach (var cid in platform.Containers.Keys) mapBuilder.Remove(cid);

            var newSnap = oldSnap with
            {
                Platforms = oldSnap.Platforms.Remove(platformId),
                ContainerToPlatform = mapBuilder.ToImmutable()
            };

            if (ReferenceEquals(
                    Interlocked.CompareExchange(ref _snapshot, newSnap, oldSnap), 
                    oldSnap))
                return true;
        }
    }

    public bool TryGetContainers(Guid platformId, [MaybeNullWhen(false)] out IReadOnlyDictionary<string, Guid> containers)
    {
        var snap = _snapshot;

        if (snap.Platforms.TryGetValue(platformId, out var platform))
        {
            containers = platform.Containers;
            return true;
        }

        containers = null;
        return false;
    }

    public bool TryGetCacheEntry(Guid platformId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry, [MaybeNullWhen(true)] out Error error)
    {
        var snap = _snapshot;

        if (snap.Platforms.TryGetValue(platformId, out var platform))
        {
            cacheEntry = new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                platform.Containers
            );
            error = null;
            return true;
        }

        cacheEntry = null;
        error = new NotFoundError("Platform is disconnected or unavailable.");
        return false;
    }

    public bool TryGetCacheEntries([MaybeNullWhen(false)] out IEnumerable<PlatformCacheEntry> cacheEntries, [MaybeNullWhen(true)] out Error error)
    {
        var snap = _snapshot;

        if (snap.Platforms.IsEmpty)
        {
            cacheEntries = null;
            error = new NotFoundError("No platform is currently connected.");
            return false;
        }

        cacheEntries = snap.Platforms.Values.Select(p =>
            new PlatformCacheEntry(
                p.Id,
                p.Address,
                p.ConnectorType,
                p.Containers));

        error = null;
        return true;
    }

    public bool TryGetPlatformWithContainer(string containerId, [MaybeNullWhen(false)] out PlatformCacheEntry cacheEntry)
    {
        var snap = _snapshot;
        var fullId = ResolveFullId(snap, containerId);

        if (fullId == null ||
            !snap.ContainerToPlatform.TryGetValue(fullId, out var pid) ||
            !snap.Platforms.TryGetValue(pid, out var platform) ||
            !platform.Containers.TryGetValue(fullId, out var dbId))
        {
            cacheEntry = null;
            return false;
        }

        cacheEntry = new PlatformCacheEntry(
            platform.Id,
            platform.Address,
            platform.ConnectorType,
            ImmutableDictionary<string, Guid>.Empty.Add(fullId, dbId)
        );

        return true;
    }

    public bool TryGetPlatformsWithContainers(string[] containersId, [MaybeNullWhen(false)] out List<PlatformCacheEntry> cacheEntries)
    {
        var snap = _snapshot;

        // platformId -> (containerId -> dbId)
        var grouped = new Dictionary<Guid, Dictionary<string, Guid>>();

        foreach (var raw in containersId)
        {
            var fullId = ResolveFullId(snap, raw);
            if (fullId == null) continue;

            if (!snap.ContainerToPlatform.TryGetValue(fullId, out var pid))
                continue;

            if (!snap.Platforms.TryGetValue(pid, out var platform))
                continue;

            if (!platform.Containers.TryGetValue(fullId, out var dbId))
                continue;

            if (!grouped.TryGetValue(pid, out var containers))
            {
                containers = [];
                grouped[pid] = containers;
            }

            containers[fullId] = dbId;
        }

        if (grouped.Count == 0)
        {
            cacheEntries = null;
            return false;
        }

        cacheEntries = new List<PlatformCacheEntry>(grouped.Count);
        foreach (var (pid, containers) in grouped)
        {
            var platform = snap.Platforms[pid];
            cacheEntries.Add(new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                containers.ToImmutableDictionary()
            ));
        }

        return true;
    }

    /// <summary>
    /// Resolves an input ID (could be 12-char or 64-char) to the actual key used in the dictionary.
    /// </summary>
    private static string? ResolveFullId(PlatformSnapshot snap, string id)
    {
        var nid = NormalizeId(id);

        if (snap.ContainerToPlatform.ContainsKey(nid))
            return nid;

        if (nid.Length >= ShortIdLength)
        {
            // In case of multiple matches (extremely rare), FirstOrDefault returns the first found.
            return snap.ContainerToPlatform.Keys
                .FirstOrDefault(k => k.StartsWith(nid, StringComparison.OrdinalIgnoreCase));
        }

        return null;
    }
}

internal sealed record PlatformSnapshot(
    ImmutableDictionary<Guid, PlatformView> Platforms,
    ImmutableDictionary<string, Guid> ContainerToPlatform
);

internal sealed record PlatformView(
    Guid Id,
    string Address,
    PlatformConnectorType ConnectorType,
    ImmutableDictionary<string, Guid> Containers
);