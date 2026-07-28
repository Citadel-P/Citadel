using Hosting.DockerClient;
using Hosting.DockerClient.Models.Containers;
using Hosting.DockerClient.Services;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.DockerClient;

public sealed class CacheServiceTests
{
    [Fact]
    public async Task Remove_FromFactory_ShouldNotDisposeHeldLock()
    {
        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        using var cache = new CacheService(memoryCache);

        var value = await cache.GetOrCreate(
            "container-1",
            _ =>
            {
                cache.Remove("container-1");
                return Task.FromResult<string?>("value");
            },
            TimeSpan.FromMinutes(1),
            TestContext.Current.CancellationToken);

        Assert.Equal("value", value);
    }

    [Fact]
    public async Task ListContainersAsync_ShouldBoundConcurrentStatsRequests()
    {
        var summaries = Enumerable.Range(0, 12)
            .Select(index => new ContainerSummary
            {
                Id = index.ToString("x64"),
                Names = [$"/container-{index}"],
                Image = "busybox:latest",
                ImageID = "sha256:image",
                Ports = [],
                Labels = new Dictionary<string, string>(),
                State = ContainerSummaryState.Running,
                Status = "Up"
            })
            .ToArray();
        var dockerClient = new Mock<IDockerClient>();
        dockerClient
            .Setup(client => client.ContainerList(
                It.IsAny<bool?>(),
                It.IsAny<int?>(),
                It.IsAny<bool?>(),
                It.IsAny<string>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(summaries);
        var cache = new ConcurrencyTrackingCacheService();
        var service = new ContainerService(
            dockerClient.Object,
            Mock.Of<IStreamService>(),
            cache,
            Mock.Of<IDockerConnection>(),
            Mock.Of<ILogger<ContainerService>>());

        await service.ListContainersAsync(
            new ListContainersCommand(All: true),
            TestContext.Current.CancellationToken);

        Assert.InRange(cache.MaximumConcurrency, 2, 4);
    }

    private sealed class ConcurrencyTrackingCacheService : ICacheService
    {
        private int currentConcurrency;
        private int maximumConcurrency;

        public int MaximumConcurrency => Volatile.Read(ref maximumConcurrency);

        public async ValueTask<T?> GetOrCreate<T>(
            string key,
            Func<CancellationToken, Task<T?>> factory,
            TimeSpan ttl,
            CancellationToken cancellationToken = default)
        {
            var current = Interlocked.Increment(ref currentConcurrency);
            UpdateMaximum(current);
            try
            {
                await Task.Delay(TimeSpan.FromMilliseconds(25), cancellationToken);
                return default;
            }
            finally
            {
                Interlocked.Decrement(ref currentConcurrency);
            }
        }

        public void Remove(string key)
        {
        }

        private void UpdateMaximum(int value)
        {
            var observed = Volatile.Read(ref maximumConcurrency);
            while (value > observed)
            {
                var previous = Interlocked.CompareExchange(
                    ref maximumConcurrency,
                    value,
                    observed);
                if (previous == observed)
                    return;

                observed = previous;
            }
        }
    }
}
