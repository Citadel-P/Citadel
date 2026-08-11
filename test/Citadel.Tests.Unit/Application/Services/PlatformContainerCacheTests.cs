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

    [Fact]
    public void ShortContainerId_ShouldBeRejectedWhenItIsAmbiguous()
    {
        var firstPlatformId = Guid.CreateVersion7();
        var secondPlatformId = Guid.CreateVersion7();
        var cache = new PlatformContainerCache();
        cache.ReplacePlatformContainers(
            firstPlatformId,
            new PlatformCacheEntry(
                firstPlatformId,
                "https://first.example",
                PlatformConnectorType.Agent,
                ImmutableDictionary<string, Guid>.Empty.Add(
                    "123456789abc0000000000000000000000000000000000000000000000000000",
                    Guid.CreateVersion7())));
        cache.ReplacePlatformContainers(
            secondPlatformId,
            new PlatformCacheEntry(
                secondPlatformId,
                "https://second.example",
                PlatformConnectorType.Agent,
                ImmutableDictionary<string, Guid>.Empty.Add(
                    "123456789abc1111111111111111111111111111111111111111111111111111",
                    Guid.CreateVersion7())));

        Assert.False(cache.TryGetPlatformWithContainer("123456789abc", out _));
    }
}
