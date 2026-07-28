using Application.Services;

namespace Tests.Unit.Application.Services;

public sealed class ImageDigestCacheTests
{
    [Fact]
    public void Set_BoundsCacheSize()
    {
        var cache = new ImageDigestCache();

        for (var index = 0; index < 4200; index++)
        {
            cache.Set(
                new ImageKey(Guid.Empty, $"repository-{index}", "latest"),
                $"sha256:{index}");
        }

        Assert.InRange(cache.Count, 3584, 4096);

        var countAfterCompaction = cache.Count;
        for (var index = 0; index < 100; index++)
        {
            cache.Set(
                new ImageKey(Guid.Empty, $"additional-repository-{index}", "latest"),
                $"sha256:additional-{index}");
        }

        Assert.Equal(countAfterCompaction + 100, cache.Count);
    }
}
