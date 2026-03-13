using System.Collections.Concurrent;

namespace Application.Services;

internal sealed class ImageDigestCache
{
    private static readonly TimeSpan EntryTtl = TimeSpan.FromDays(1);
    private readonly ConcurrentDictionary<ImageKey, ImageDigestEntry> _cache = new();

    public bool TryGet(ImageKey key, out ImageDigestEntry entry)
    {
        if (!_cache.TryGetValue(key, out entry))
            return false;

        if (DateTime.UtcNow - entry.CheckedAt <= EntryTtl)
            return true;

        _cache.TryRemove(key, out _);
        entry = default;
        return false;
    }

    public void Set(ImageKey key, string digest)
        => _cache[key] = new ImageDigestEntry(digest, DateTime.UtcNow);
}

internal readonly record struct ImageKey(
    Guid RegistryId,
    string Repository,
    string Tag);

internal readonly record struct ImageDigestEntry(
    string Digest,
    DateTime CheckedAt);
