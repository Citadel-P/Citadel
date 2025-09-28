using Application.TaskJobs;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public class BackgroundTaskQueueTests
{
    [Fact]
    public void Enqueue_Null_ThrowsArgumentNullException()
    {
        var q = new BackgroundTaskQueue();

        Assert.Throws<ArgumentNullException>(() => q.Enqueue(null!));
    }

    [Fact]
    public async Task Enqueue_Dequeue_Roundtrip_ReturnsSameDelegate()
    {
        var q = new BackgroundTaskQueue();

        Func<CancellationToken, Task> work = _ => Task.CompletedTask;
        q.Enqueue(work);

        var dequeued = await q.DequeueAsync(CancellationToken.None);
        Assert.Same(work, dequeued);
    }

    [Fact]
    public async Task DequeueAsync_WhenCancelled_ThrowsTaskCanceledException()
    {
        var q = new BackgroundTaskQueue();

        using var cts = new CancellationTokenSource();
        cts.Cancel();

        await Assert.ThrowsAsync<TaskCanceledException>(() => q.DequeueAsync(cts.Token).AsTask());
    }

    [Fact]
    public async Task QueuedHostedService_ExecutesEnqueuedWorkItem()
    {
        var q = new BackgroundTaskQueue();
        var logger = new Mock<ILogger<QueuedHostedService>>();

        var tcs = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
        int executed = 0;

        Func<CancellationToken, Task> work = ct =>
        {
            Interlocked.Increment(ref executed);
            tcs.TrySetResult(true);
            return Task.CompletedTask;
        };

        // Start the hosted service with a cancellation token we can cancel later.
        using var cts = new CancellationTokenSource();
        var service = new QueuedHostedService(q, logger.Object);
        // StartAsync will schedule ExecuteAsync in background.
        var start = service.StartAsync(cts.Token);

        // Enqueue the work item and wait for it to run.
        q.Enqueue(work);

        // Wait for work to be executed (or fail the test after timeout).
        var finished = await Task.WhenAny(tcs.Task, Task.Delay(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken));
        Assert.True(finished == tcs.Task, "Background work was not executed in time.");
        Assert.Equal(1, executed);

        // Stop the background service by cancelling the stopping token and await StopAsync.
        cts.Cancel();
        await service.StopAsync(CancellationToken.None);
    }
}
