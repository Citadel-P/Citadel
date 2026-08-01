using Application.Services;

namespace Tests.Unit.Application.Services;

public class SyncBarrierTests
{
    private readonly Guid _id = Guid.NewGuid();
    private readonly Guid _otherId = Guid.NewGuid();

    // Dummy job marker types
    private class JobA { }
    private class JobB { }
    private class JobC { }

    [Fact]
    public async Task WaitForAsync_CompletesAfterMarkSynced()
    {
        var barrier = new SyncBarrier();

        var waiting = Task.Run(() => barrier.WaitForAsync<JobA>(_id).AsTask(), TestContext.Current.CancellationToken);

        await Task.Delay(50, TestContext.Current.CancellationToken);
        Assert.False(waiting.IsCompleted);

        barrier.MarkSynced<JobA>(_id);

        var finished = await Task.WhenAny(waiting, Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken));
        Assert.Equal(waiting, finished);

        await waiting;
    }

    [Fact]
    public async Task WaitForAsync_WithCancellation_ThrowsOperationCanceledException()
    {
        var barrier = new SyncBarrier();
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(100));

        await Assert.ThrowsAsync<TaskCanceledException>(() =>
            barrier.WaitForAsync<JobC>(_id, cts.Token).AsTask());
    }

    [Fact]
    public async Task DifferentJobs_DoNotUnblockEachOther()
    {
        var barrier = new SyncBarrier();

        var waitA = barrier.WaitForAsync<JobA>(_id).AsTask();
        var waitB = barrier.WaitForAsync<JobB>(_id).AsTask();

        barrier.MarkSynced<JobA>(_id);

        await Task.Delay(100);
        Assert.True(waitA.IsCompleted);
        Assert.False(waitB.IsCompleted);

        barrier.MarkSynced<JobB>(_id);
        await waitB;
    }

    [Fact]
    public async Task DifferentPlatformIds_DoNotUnblockEachOther()
    {
        var barrier = new SyncBarrier();

        var wait1 = barrier.WaitForAsync<JobA>(_id).AsTask();
        var wait2 = barrier.WaitForAsync<JobA>(_otherId).AsTask();

        barrier.MarkSynced<JobA>(_id);

        await Task.Delay(100);
        Assert.True(wait1.IsCompleted);
        Assert.False(wait2.IsCompleted);

        barrier.MarkSynced<JobA>(_otherId);
        await wait2;
    }

    [Fact]
    public async Task MarkSynced_BeforeWait_CompletesImmediately()
    {
        var barrier = new SyncBarrier();

        barrier.MarkSynced<JobA>(_id);

        var wait = barrier.WaitForAsync<JobA>(_id).AsTask();
        Assert.True(wait.IsCompleted, "Task should already be completed immediately.");

        await wait;
    }

    [Fact]
    public async Task MultipleWaiters_AreAllReleased()
    {
        var barrier = new SyncBarrier();

        var wait1 = barrier.WaitForAsync<JobA>(_id).AsTask();
        var wait2 = barrier.WaitForAsync<JobA>(_id).AsTask();
        var wait3 = barrier.WaitForAsync<JobA>(_id).AsTask();

        barrier.MarkSynced<JobA>(_id);

        await Task.WhenAll(wait1, wait2, wait3);
    }

    [Fact]
    public async Task RemovePlatform_ReleasesWaitersAndRemovesEveryBarrierForPlatform()
    {
        var barrier = new SyncBarrier();
        var waitA = barrier.WaitForAsync<JobA>(_id).AsTask();
        var waitB = barrier.WaitForAsync<JobB>(_id).AsTask();
        barrier.MarkSynced<JobA>(_otherId);

        barrier.RemovePlatform(_id);

        await Task.WhenAll(waitA, waitB);
        Assert.Equal(1, barrier.Count);
        Assert.True(barrier.WaitForAsync<JobA>(_otherId).IsCompletedSuccessfully);
    }

    [Fact]
    public async Task WaitForAsync_AfterPlatformRemoval_CompletesImmediately()
    {
        var barrier = new SyncBarrier();

        barrier.RemovePlatform(_id);

        var wait = barrier.WaitForAsync<JobA>(_id).AsTask();
        Assert.True(wait.IsCompletedSuccessfully);
        await wait;
        Assert.Equal(0, barrier.Count);
    }
}
