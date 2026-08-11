using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class ContainerStatsMemoryTests
{
    [Fact]
    public void ContainersStatBatch_ShouldReturnItsListOnlyOnce()
    {
        var returnCount = 0;
        var batch = new ContainersStatBatch(
            Guid.CreateVersion7(),
            [],
            _ => returnCount++);

        batch.Release();
        batch.Release();

        Assert.Equal(1, returnCount);
    }

    [Fact]
    public void ListPoolPolicy_ShouldRejectOversizedBackingArrays()
    {
        var policy = new ContainerStatsListPooledObjectPolicy();
        var stats = new List<ContainerStat>(ContainerStatsListPooledObjectPolicy.MaxRetainedCapacity + 1);

        var retained = policy.Return(stats);

        Assert.False(retained);
        Assert.Empty(stats);
    }

    [Fact]
    public void ListPoolPolicy_ShouldClearAndRetainNormalLists()
    {
        var policy = new ContainerStatsListPooledObjectPolicy();
        var stats = new List<ContainerStat>(1)
        {
            new(
                Guid.CreateVersion7(),
                MemoryActive: 1,
                MemoryCache: 2,
                CpuUsage: 3,
                MemoryLimit: 4,
                RxBytes: 5,
                TxBytes: 6,
                Created: 7)
        };

        var retained = policy.Return(stats);

        Assert.True(retained);
        Assert.Empty(stats);
    }

    [Fact]
    public async Task ContainerStatsBatchWorkItem_ShouldPropagatePersistenceFailure()
    {
        var platformId = Guid.CreateVersion7();
        var containerId = Guid.CreateVersion7();
        var stat = new ContainerStat(
            containerId,
            MemoryActive: 1,
            MemoryCache: 2,
            CpuUsage: 3,
            MemoryLimit: 4,
            RxBytes: 5,
            TxBytes: 6,
            Created: 7);
        IReadOnlyDictionary<string, Guid> containers =
            new Dictionary<string, Guid> { ["docker-id"] = containerId };
        var cache = new Mock<IPlatformContainerCache>();
        cache
            .Setup(value => value.TryGetContainers(platformId, out containers))
            .Returns(true);
        var statsRepository = new Mock<IContainerStatRepository>();
        statsRepository
            .Setup(repository => repository.BulkInsertAsync(
                It.IsAny<IEnumerable<ContainerStat>>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("database unavailable"));
        var swarmServiceStatsRepository = new Mock<ISwarmServiceStatRepository>();
        swarmServiceStatsRepository
            .Setup(repository => repository.GetAttributionsAsync(
                It.IsAny<Guid[]>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(value => value.ContainerStats).Returns(statsRepository.Object);
        uow.SetupGet(value => value.SwarmServiceStats).Returns(swarmServiceStatsRepository.Object);
        var workItem = new ContainerStatsBatchWorkItem(
            new Dictionary<Guid, List<ContainerStat>>
            {
                [platformId] = [stat]
            },
            new Dictionary<Guid, SwarmNodeStatsSource>(),
            cache.Object,
            NullLogger.Instance);

        var exception = await Assert.ThrowsAsync<InvalidOperationException>(() =>
            workItem.ExecuteAsync(uow.Object, TestContext.Current.CancellationToken));

        Assert.Equal("database unavailable", exception.Message);
        uow.Verify(
            value => value.CommitAsync(It.IsAny<CancellationToken>()),
            Times.Never);
    }
}
