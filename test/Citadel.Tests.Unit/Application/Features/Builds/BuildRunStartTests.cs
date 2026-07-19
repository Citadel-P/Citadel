using Application.Features.Builds.Commands;
using Application.Features.Builds.Models;
using Application.Services;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Builds;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common.Abstraction;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Builds;

public sealed class BuildRunStartTests
{
    [Fact]
    public async Task QueueBuildRun_ShouldCreateSingleRunAndNotifyAfterCommit()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries);
        var runStream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);
        var projectStream = new Mock<IBuildProjectStreamManager>(MockBehavior.Strict);
        BuildRun? capturedRun = null;

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        gitRepositories
            .Setup(x => x.GetAsync(project.GitRepositoryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        platforms
            .Setup(x => x.GetInfoAsync(project.PlatformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        registries
            .Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildRuns
            .Setup(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((run, _) => capturedRun = run)
            .ReturnsAsync(1);
        buildProjects
            .Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .Callback<Guid, Guid, CancellationToken>((_, runId, _) => project.MarkProcessing(runId, DateTimeOffset.UtcNow))
            .ReturnsAsync(1);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        runStream
            .Setup(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), "create"))
            .Returns(Task.CompletedTask);
        projectStream
            .Setup(x => x.SendBuildProjectInfo(It.IsAny<BuildProject>(), "update", It.IsAny<BuildRun?>()))
            .Returns(Task.CompletedTask);
        var handler = new QueueBuildRunHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(actorId),
            projectStream.Object,
            runStream.Object);

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var queued, out var error), error?.Message);
        Assert.NotNull(capturedRun);
        Assert.Equal(capturedRun.Id, queued.Run.Id);
        Assert.Equal(BuildRunStatus.Queued, queued.Run.Status);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Once);
        buildProjects.Verify(x => x.MarkProcessingAsync(project.Id, capturedRun.Id, It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        runStream.Verify(x => x.SendBuildRunInfo(capturedRun, "create"), Times.Once);
        projectStream.Verify(x => x.SendBuildProjectInfo(project, "update", capturedRun), Times.Once);
    }

    [Fact]
    public async Task QueueBuildRun_ShouldRejectWhenProjectAlreadyHasActiveRun()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(
            buildProjects,
            buildRuns,
            new Mock<IGitReposRepository>(MockBehavior.Strict),
            new Mock<IPlatformRepository>(MockBehavior.Strict),
            new Mock<IRegistryRepository>(MockBehavior.Strict));
        var handler = new QueueBuildRunHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(actorId),
            Mock.Of<IBuildProjectStreamManager>(),
            Mock.Of<IBuildRunStreamManager>());

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("already has an active run", error.Message);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        buildProjects.Verify(x => x.MarkProcessingAsync(It.IsAny<Guid>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task QueueBuildRun_ShouldNotCommitOrNotify_WhenMarkProcessingLosesRace()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries);
        var runStream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);
        var projectStream = new Mock<IBuildProjectStreamManager>(MockBehavior.Strict);
        var handler = new QueueBuildRunHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(actorId),
            projectStream.Object,
            runStream.Object);

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        gitRepositories
            .Setup(x => x.GetAsync(project.GitRepositoryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        platforms
            .Setup(x => x.GetInfoAsync(project.PlatformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        registries
            .Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildRuns
            .Setup(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        buildProjects
            .Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("already has an active run", error.Message);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        runStream.Verify(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), It.IsAny<string>()), Times.Never);
        projectStream.Verify(x => x.SendBuildProjectInfo(It.IsAny<BuildProject>(), It.IsAny<string>(), It.IsAny<BuildRun?>()), Times.Never);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldNotStartProcess_WhenRunWasAlreadyClaimed()
    {
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildRunLogs = new Mock<IBuildRunLogRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        var runner = new Mock<IBuildProcessRunner>(MockBehavior.Strict);
        var service = new BuildRunExecutionService(
            unitOfWork.Object,
            Mock.Of<IRepoCacheManager>(),
            runner.Object,
            new BuildRunCoordinator(),
            Mock.Of<ISecretValueProtector>(),
            Mock.Of<IExternalSecretProviderClient>(),
            Mock.Of<IBuildProjectStreamManager>(),
            Mock.Of<IBuildRunStreamManager>(),
            Mock.Of<IBuildRunRetentionService>());
        var runId = Guid.CreateVersion7();

        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns.Object);
        unitOfWork.SetupGet(x => x.BuildRunLogs).Returns(buildRunLogs.Object);
        unitOfWork.SetupGet(x => x.BuildProjects).Returns(buildProjects.Object);
        buildRuns
            .Setup(x => x.TryClaimAsync(runId, It.IsAny<DateTimeOffset>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((BuildRun?)null);

        var result = await service.ExecuteAsync(runId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        runner.Verify(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        buildRunLogs.Verify(x => x.AddAsync(It.IsAny<BuildRunLogEntry>(), It.IsAny<CancellationToken>()), Times.Never);
        buildProjects.Verify(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>(), It.IsAny<bool>()), Times.Never);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        Mock<IBuildProjectRepository> buildProjects,
        Mock<IBuildRunRepository> buildRuns,
        Mock<IGitReposRepository> gitRepositories,
        Mock<IPlatformRepository> platforms,
        Mock<IRegistryRepository> registries)
    {
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.BuildProjects).Returns(buildProjects.Object);
        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns.Object);
        unitOfWork.SetupGet(x => x.GitRepositories).Returns(gitRepositories.Object);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(x => x.Registries).Returns(registries.Object);
        return unitOfWork;
    }

    private static BuildProject CreateProject(Guid actorId)
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
            BuildProject.DefaultTimeoutSeconds,
            BuildProject.DefaultRetentionRunCount,
            actorId);

    private static GitRepository CreateRepository(Guid id, Guid actorId)
        => GitRepository.FromPersistence(
            id,
            "api",
            null,
            "https://example.test/citadel/api",
            "main",
            GitReposStatus.Created,
            null,
            DateTime.UtcNow,
            actorId);

    private static PlatformConnectionInfo CreatePlatform(Guid id)
        => new(
            id,
            "local",
            "unix:///var/run/docker.sock",
            PlatformConnectorType.Local);

    private static Registry CreateRegistry(Guid id, Guid actorId)
        => Registry.FromPersistence(
            id,
            "local-registry",
            null,
            RegistryStatus.Active,
            "registry.example.test",
            DateTime.UtcNow,
            actorId,
            new CustomRegistry());

    private static IUserContextAccessor CreateUserContextAccessor(Guid actorId)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.UserId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.ActorId).Returns(actorId);
        user.SetupGet(x => x.IsAdmin).Returns(true);
        user.SetupGet(x => x.IsAuthenticated).Returns(true);
        user.SetupGet(x => x.Roles).Returns(["admin"]);

        var accessor = new Mock<IUserContextAccessor>();
        accessor.SetupGet(x => x.Current).Returns(user.Object);
        return accessor.Object;
    }
}
