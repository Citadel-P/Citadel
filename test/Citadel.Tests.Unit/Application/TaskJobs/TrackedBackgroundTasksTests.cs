using Application.TaskJobs;
using System.Diagnostics;

namespace Tests.Unit.Application.TaskJobs;

public sealed class TrackedBackgroundTasksTests
{
    [Fact]
    public async Task DrainAsync_ShouldRespectShutdownDeadline()
    {
        var tasks = new TrackedBackgroundTasks();
        var neverCompletes = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        tasks.Add(neverCompletes.Task);
        using var deadline = new CancellationTokenSource(TimeSpan.FromMilliseconds(100));
        var stopwatch = Stopwatch.StartNew();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            tasks.DrainAsync(deadline.Token));

        Assert.True(stopwatch.Elapsed < TimeSpan.FromSeconds(2));
        neverCompletes.TrySetCanceled();
    }
}
