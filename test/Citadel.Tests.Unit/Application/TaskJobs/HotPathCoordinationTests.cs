using Application.TaskJobs;

namespace Tests.Unit.Application.TaskJobs;

public sealed class HotPathCoordinationTests
{
    [Fact]
    public void GitRepoSyncInFlightTracker_DeduplicatesBranchUntilRemoved()
    {
        var tracker = new GitRepoSyncInFlightTracker();
        var repositoryId = Guid.NewGuid();

        Assert.True(tracker.TryAdd(repositoryId, "Main"));
        Assert.False(tracker.TryAdd(repositoryId, " main "));

        tracker.Remove(repositoryId, "MAIN");

        Assert.True(tracker.TryAdd(repositoryId, "main"));
    }

    [Fact]
    public void WorkerPollingDelay_BacksOffWhenIdleAndResetsAfterDispatch()
    {
        var minimum = TimeSpan.FromSeconds(2);

        var firstIdle = WorkerPollingDelay.Next(minimum, minimum, dispatchedWork: false);
        var secondIdle = WorkerPollingDelay.Next(firstIdle, minimum, dispatchedWork: false);
        var capped = WorkerPollingDelay.Next(TimeSpan.FromSeconds(10), minimum, dispatchedWork: false);

        Assert.Equal(TimeSpan.FromSeconds(4), firstIdle);
        Assert.Equal(TimeSpan.FromSeconds(8), secondIdle);
        Assert.Equal(TimeSpan.FromSeconds(10), capped);
        Assert.Equal(minimum, WorkerPollingDelay.Next(secondIdle, minimum, dispatchedWork: true));
    }
}
