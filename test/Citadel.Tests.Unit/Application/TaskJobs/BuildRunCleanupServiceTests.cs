using Application.Configs;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Resources;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
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

    [Fact]
    public async Task PruneAsync_ShouldDeleteRunsBeyondProjectRetentionAndStreamDeletes()
    {
        var project = CreateProject(retentionRunCount: 2);
        var deletedRun = CreateRun(project);
        deletedRun.MarkPreparing(DateTimeOffset.UtcNow);
        deletedRun.MarkRunning(DateTimeOffset.UtcNow);
        deletedRun.CompleteSucceeded("sha256:old", deletedRun.ImageReferences, 0, DateTimeOffset.UtcNow);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildRuns.Object, buildProjects.Object);
        var stream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.DeleteTerminalRunsBeyondRetentionAsync(project.Id, 2, It.IsAny<CancellationToken>()))
            .ReturnsAsync([deletedRun]);
        stream
            .Setup(x => x.SendBuildRunInfo(deletedRun, "delete"))
            .Returns(Task.CompletedTask);

        var service = new BuildRunRetentionService(unitOfWork.Object, stream.Object);

        await service.PruneAsync(project.Id, TestContext.Current.CancellationToken);

        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        stream.Verify(x => x.SendBuildRunInfo(deletedRun, "delete"), Times.Once);
    }

    [Fact]
    public async Task PruneAsync_WhenNoRunsDeleted_ShouldNotCommitOrStream()
    {
        var project = CreateProject(retentionRunCount: 20);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildRuns.Object, buildProjects.Object);
        var stream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.DeleteTerminalRunsBeyondRetentionAsync(project.Id, 20, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var service = new BuildRunRetentionService(unitOfWork.Object, stream.Object);

        await service.PruneAsync(project.Id, TestContext.Current.CancellationToken);

        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        stream.VerifyNoOtherCalls();
    }

    private static BuildRunCleanupService CreateService(DateTimeOffset now, BuildOptions options)
    {
        return new BuildRunCleanupService(
            Options.Create(options),
            new FixedTimeProvider(now),
            NullLogger<BuildRunCleanupService>.Instance);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        IBuildRunRepository buildRuns,
        IBuildProjectRepository? buildProjects = null)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns);
        if (buildProjects is not null)
            unitOfWork.SetupGet(x => x.BuildProjects).Returns(buildProjects);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static BuildProject CreateProject(int retentionRunCount)
        => new(
            "api-image",
            null,
            true,
            Guid.CreateVersion7(),
            "main",
            ".",
            "Dockerfile",
            null,
            [],
            [],
            Guid.CreateVersion7(),
            Guid.CreateVersion7(),
            "citadel/api",
            ["{branch}-{shortSha}"],
            null,
            BuildProject.DefaultTimeoutSeconds,
            retentionRunCount,
            Guid.CreateVersion7());

    private static BuildRun CreateRun(BuildProject project)
        => new(
            project.Id,
            project.Name,
            project.GitRepositoryId,
            "api",
            project.Branch,
            "abcdef1234567890",
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [],
            new BuildPlatformSnapshot(project.PlatformId, "local", "unix:///var/run/docker.sock", PlatformConnectorType.Local),
            new BuildRegistrySnapshot(project.RegistryId, "registry", "registry.example.test"),
            project.ImageRepository,
            project.TagTemplates,
            ["registry.example.test/citadel/api:main-abcdef123456"],
            BuildRunTrigger.Manual,
            null,
            project.CreatedByActorId,
            project.TimeoutSeconds);

    private sealed class FixedTimeProvider(DateTimeOffset now) : TimeProvider
    {
        public override DateTimeOffset GetUtcNow() => now;
    }
}
