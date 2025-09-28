using Application.Services;

namespace Tests.Unit.Application.Services;

public class SyncBarrierTests
{
    // Dummy job marker types
    private class JobA { }
    private class JobB { }
    private class JobC { }

    [Fact]
    public async Task WaitForAsync_CompletesAfterMarkSynced()
    {
        var barrier = new SyncBarrier();

        // Start waiting on JobA in the background
        var waiting = Task.Run(() => barrier.WaitForAsync<JobA>().AsTask(), TestContext.Current.CancellationToken);

        // Ensure the waiter is not yet completed
        await Task.Delay(50, TestContext.Current.CancellationToken);
        Assert.False(waiting.IsCompleted);

        // Mark JobA synced and assert waiter completes promptly
        barrier.MarkSynced<JobA>();

        var finished = await Task.WhenAny(waiting, Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken));
        Assert.Equal(waiting, finished);
        await waiting; // propagate any exception (there should be none)
    }

    [Fact]
    public async Task WaitForAllAsync_WaitsForBothJobs()
    {
        var barrier = new SyncBarrier();

        // Start waiting for both JobA and JobB
        var waitAll = Task.Run(() => barrier.WaitForAllAsync<JobA, JobB>(TestContext.Current.CancellationToken).AsTask(), TestContext.Current.CancellationToken);

        // Mark only JobA and ensure waitAll is not completed yet
        barrier.MarkSynced<JobA>();
        await Task.Delay(50, TestContext.Current.CancellationToken);
        Assert.False(waitAll.IsCompleted);

        // Now mark JobB and ensure completion
        barrier.MarkSynced<JobB>();
        var finished = await Task.WhenAny(waitAll, Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken));
        Assert.Equal(waitAll, finished);
        await waitAll;
    }

    [Fact]
    public void WaitForAsync_AlreadyMarked_CompletesImmediately()
    {
        var barrier = new SyncBarrier();

        // Mark before waiting
        barrier.MarkSynced<JobA>();

        var vt = barrier.WaitForAsync<JobA>(TestContext.Current.CancellationToken);
        // ValueTask should be already completed
        Assert.True(vt.IsCompleted);
    }

    [Fact]
    public async Task WaitForAsync_WithCancellation_ThrowsOperationCanceledException()
    {
        var barrier = new SyncBarrier();
        using var cts = new CancellationTokenSource(TimeSpan.FromMilliseconds(100));

        // Awaiting should observe cancellation and throw
        await Assert.ThrowsAsync<TaskCanceledException>(() => barrier.WaitForAsync<JobC>(cts.Token).AsTask());
    }
}