using Application.Services;
using Domain;
using Domain.Contracts.Resources;
using System.Collections.Immutable;

namespace Tests.Unit.Application.Services;

public sealed class PlatformContainerCacheTests
{
    [Fact]
    public void MutationVersion_ShouldNotResetAfterPlatformEvictionAndReconnect()
    {
        var platformId = Guid.CreateVersion7();
        var cache = new PlatformContainerCache();
        var entry = new PlatformCacheEntry(
            platformId,
            "https://platform.example",
            PlatformConnectorType.Agent,
            ImmutableDictionary<string, Guid>.Empty);

        cache.ReplacePlatformContainers(platformId, entry);
        var beforeEviction = cache.GetMutationVersion(platformId);
        Assert.True(cache.EvictPlatform(platformId));
        cache.ReplacePlatformContainers(platformId, entry);

        Assert.True(cache.GetMutationVersion(platformId) > beforeEviction);
    }
}
