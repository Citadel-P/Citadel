using System.Collections.Concurrent;

namespace Application.Services;

internal sealed class ImageDigestCache(TimeProvider? timeProvider = null)
{
    private static readonly TimeSpan EntryTtl = TimeSpan.FromDays(1);
    private const int MaxEntries = 4096;
    private const int LowWaterMark = 3584;
    private readonly ConcurrentDictionary<ImageKey, ImageDigestEntry> _cache = new();
    private readonly TimeProvider _timeProvider = timeProvider ?? TimeProvider.System;
    private int _setsSinceCompaction;
    private int _compacting;

    internal int Count => _cache.Count;

    public bool TryGet(ImageKey key, out ImageDigestEntry entry)
    {
        if (!_cache.TryGetValue(key, out entry))
            return false;

        if (_timeProvider.GetUtcNow().UtcDateTime - entry.CheckedAt <= EntryTtl)
            return true;

        _cache.TryRemove(key, out _);
        entry = default;
        return false;
    }

    public void Set(ImageKey key, string digest)
    {
        var now = _timeProvider.GetUtcNow().UtcDateTime;
        _cache[key] = new ImageDigestEntry(digest, now);

        if (_cache.Count > MaxEntries
            || (Interlocked.Increment(ref _setsSinceCompaction) & 255) == 0)
        {
            TryCompact(now);
        }
    }

    private void TryCompact(DateTime now)
    {
        if (Interlocked.CompareExchange(ref _compacting, 1, 0) != 0)
            return;

        try
        {
            Compact(now);
        }
        finally
        {
            Volatile.Write(ref _compacting, 0);
        }
    }

    private void Compact(DateTime now)
    {
        foreach (var (key, entry) in _cache)
        {
            if (now - entry.CheckedAt > EntryTtl)
                RemoveEntry(key, entry);
        }

        var excess = _cache.Count - MaxEntries;
        if (excess <= 0)
            return;

        var removeCount = _cache.Count - LowWaterMark;
        foreach (var entry in _cache
                     .OrderBy(static pair => pair.Value.CheckedAt)
                     .Take(removeCount))
        {
            RemoveEntry(entry.Key, entry.Value);
        }
    }

    private void RemoveEntry(ImageKey key, ImageDigestEntry entry)
        => ((ICollection<KeyValuePair<ImageKey, ImageDigestEntry>>)_cache)
            .Remove(new KeyValuePair<ImageKey, ImageDigestEntry>(key, entry));
}

internal readonly record struct ImageKey(
    Guid RegistryId,
    string Repository,
    string Tag);

internal readonly record struct ImageDigestEntry(
    string Digest,
    DateTime CheckedAt);
