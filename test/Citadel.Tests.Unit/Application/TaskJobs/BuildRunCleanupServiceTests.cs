using Application.Configs;
using Application.Services.Builds;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class BuildRunCleanupServiceTests
{
    [Fact]
    public async Task RemoveExpiredRunsAsync_ShouldDeleteRunsOlderThanConfiguredRetention()
    {
        var now = new DateTimeOffset(2026, 7, 19, 10, 0, 0, TimeSpan.Zero);
        DateTime? completedBefore = null;
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns
            .Setup(x => x.RemoveCompletedOlderThanAsync(It.IsAny<DateTime>(), It.IsAny<CancellationToken>()))
            .Callback<DateTime, CancellationToken>((threshold, _) => completedBefore = threshold)
            .ReturnsAsync(3);
        var unitOfWork = CreateUnitOfWork(buildRuns.Object);
        var service = CreateService(now, new BuildOptions { RunRetentionDays = 90 });

        var deleted = await service.RemoveExpiredRunsAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(3, deleted);
        Assert.Equal(now.AddDays(-90).UtcDateTime, completedBefore);
        buildRuns.Verify(x => x.RemoveCompletedOlderThanAsync(It.IsAny<DateTime>(), It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task RemoveExpiredRunsAsync_WhenDisabled_ShouldNotDeleteRuns()
    {
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildRuns.Object);
        var service = new BuildRunCleanupService(
            Options.Create(new BuildOptions { RunCleanupEnabled = false }),
            new FixedTimeProvider(DateTimeOffset.UtcNow),
            NullLogger<BuildRunCleanupService>.Instance);

        var deleted = await service.RemoveExpiredRunsAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        Assert.Equal(0, deleted);
        buildRuns.VerifyNoOtherCalls();
    }

    private static BuildRunCleanupService CreateService(DateTimeOffset now, BuildOptions options)
    {
        return new BuildRunCleanupService(
            Options.Create(options),
            new FixedTimeProvider(now),
            NullLogger<BuildRunCleanupService>.Instance);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(IBuildRunRepository buildRuns)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private sealed class FixedTimeProvider(DateTimeOffset now) : TimeProvider
    {
        public override DateTimeOffset GetUtcNow() => now;
    }
}
