using Application.Services.Builds;

namespace Tests.Unit.Application.Services;

public sealed class BuildRunCoordinatorTests
{
    [Fact]
    public void Register_ShouldRejectDuplicateRegistration()
    {
        var coordinator = new BuildRunCoordinator();
        var runId = Guid.NewGuid();

        coordinator.Register(runId);

        Assert.Throws<InvalidOperationException>(() => coordinator.Register(runId));
    }

    [Fact]
    public void Cancel_ShouldCancelRegisteredRun()
    {
        var coordinator = new BuildRunCoordinator();
        var runId = Guid.NewGuid();

        var cts = coordinator.Register(runId);

        Assert.True(coordinator.Cancel(runId));
        Assert.True(cts.IsCancellationRequested);
    }

    [Fact]
    public void Register_ShouldReturnCancelledToken_WhenCancelArrivesBeforeRegistration()
    {
        var coordinator = new BuildRunCoordinator();
        var runId = Guid.NewGuid();

        Assert.False(coordinator.Cancel(runId));

        var cts = coordinator.Register(runId);

        Assert.True(cts.IsCancellationRequested);
    }

    [Fact]
    public void Unregister_ShouldClearPendingCancellation()
    {
        var coordinator = new BuildRunCoordinator();
        var runId = Guid.NewGuid();

        Assert.False(coordinator.Cancel(runId));

        coordinator.Unregister(runId);
        var cts = coordinator.Register(runId);

        Assert.False(cts.IsCancellationRequested);
    }

    [Fact]
    public void Unregister_ShouldAllowFreshRegistration()
    {
        var coordinator = new BuildRunCoordinator();
        var runId = Guid.NewGuid();

        coordinator.Register(runId);

        coordinator.Unregister(runId);

        var cts = coordinator.Register(runId);

        Assert.False(cts.IsCancellationRequested);
    }
}
