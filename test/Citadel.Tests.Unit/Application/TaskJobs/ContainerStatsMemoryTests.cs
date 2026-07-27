using Application.TaskJobs;
using Domain.Entities;

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
}
