using Application.Features.Builds.Commands;
using Application.Features.Builds.Models;
using Application.Services;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common.Abstraction;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using System.Runtime.CompilerServices;

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
            runStream.Object,
            Mock.Of<IActivityStreamManager>());

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
            Mock.Of<IBuildRunStreamManager>(),
            Mock.Of<IActivityStreamManager>());

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
            runStream.Object,
            Mock.Of<IActivityStreamManager>());

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
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
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
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IBuildRunRetentionService>(),
            NullLogger<BuildRunExecutionService>.Instance);
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

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldSyncBuildPushPersistLogsAndMarkProjectIdle()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        BuildProcessCommand? processCommand = null;
        var persistedLogs = new List<BuildRunLogEntry>();
        var statusUpdates = new List<BuildRunStatus>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abcdef1234567890", Success: true));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildProcessCommand, CancellationToken>((command, _) => processCommand = command)
            .Returns(BuildEvents(
                new BuildProcessEvent(BuildProcessStream.StdOut, "Step 1/1 : FROM scratch"),
                new BuildProcessEvent(BuildProcessStream.StdOut, "Pushed registry.example.test/citadel/api:main-abcdef123456"),
                new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((updated, _) => statusUpdates.Add(updated.Status))
            .ReturnsAsync(1);
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(processCommand);
        Assert.Equal(repository.GetCachePath(), processCommand.WorkingDirectory);
        Assert.Equal(Path.GetFullPath(repository.GetCachePath()), Path.GetFullPath(processCommand.ContextPath));
        Assert.Equal(Path.Combine(Path.GetFullPath(repository.GetCachePath()), "Dockerfile"), processCommand.DockerfilePath);
        Assert.Equal(PlatformConnectorType.Local, processCommand.PlatformConnectorType);
        Assert.Equal("registry.example.test/citadel/api:main-abcdef123456", Assert.Single(processCommand.ImageReferences));
        Assert.Contains(BuildRunStatus.Running, statusUpdates);
        Assert.Contains(BuildRunStatus.Succeeded, statusUpdates);
        Assert.Equal(BuildRunStatus.Succeeded, run.Status);
        Assert.Equal("sha256:abc", run.ImageDigest);
        Assert.Contains(persistedLogs, log => log.Stream == "stdout" && log.Message.Contains("FROM scratch", StringComparison.Ordinal));
        Assert.Contains(persistedLogs, log => log.Stream == "system" && log.Message.Contains("completed successfully", StringComparison.Ordinal));
        context.BuildProjects.Verify(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()), Times.Once);
        context.Retention.Verify(x => x.PruneAsync(project.Id, It.IsAny<CancellationToken>()), Times.Once);
        context.BuildRunStream.Verify(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), It.IsAny<string>()), Times.AtLeast(2));
        context.ProjectStream.Verify(x => x.SendBuildProjectInfo(project, "update", It.IsAny<BuildRun?>()), Times.AtLeastOnce);
    }

    [Fact]
    public async Task CancelBuildRun_ShouldSignalRunningExecutionAndPersistCancelledStatus()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        PrepareRepositoryCache(repository);

        var coordinator = new BuildRunCoordinator();
        var context = CreateExecutionContext(project, repository, platform, registry, run, coordinator);
        var runnerStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var runnerCancelled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var statusUpdates = new List<BuildRunStatus>();
        var persistedLogs = new List<BuildRunLogEntry>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(GitOperation.Pull, "abcdef1234567890", Success: true));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns((BuildProcessCommand _, CancellationToken ct) => CancellableBuild(runnerStarted, runnerCancelled, ct));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((updated, _) => statusUpdates.Add(updated.Status))
            .ReturnsAsync(1);
        context.BuildRuns
            .Setup(x => x.CancelQueuedOrRunningAsync(run.Id, It.IsAny<DateTimeOffset>(), "Build run cancelled.", It.IsAny<CancellationToken>()))
            .ReturnsAsync(() =>
            {
                var cancelled = CreateRun(project, repository, platform, registry, actorId, run.Id);
                cancelled.MarkPreparing(DateTimeOffset.UtcNow);
                cancelled.MarkRunning(DateTimeOffset.UtcNow);
                cancelled.Cancel(DateTimeOffset.UtcNow);
                return cancelled;
            });
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        var cancelHandler = new CancelBuildRunHandler(
            context.UnitOfWork.Object,
            coordinator,
            context.ProjectStream.Object,
            context.BuildRunStream.Object,
            context.Retention.Object);

        var executionTask = context.Service.ExecuteAsync(run.Id, CancellationToken.None).AsTask();
        await runnerStarted.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        var cancelResult = await cancelHandler.Handle(new CancelBuildRun(run.Id), TestContext.Current.CancellationToken);
        var executionResult = await executionTask.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);

        Assert.True(cancelResult.IsSuccess());
        Assert.True(executionResult.IsSuccess());
        await runnerCancelled.Task.WaitAsync(TimeSpan.FromSeconds(5), TestContext.Current.CancellationToken);
        Assert.Equal(BuildRunStatus.Cancelled, run.Status);
        Assert.Contains(BuildRunStatus.Cancelled, statusUpdates);
        Assert.Contains(persistedLogs, log => log.Stream == "stderr" && log.Message.Contains("cancelled", StringComparison.OrdinalIgnoreCase));
        context.BuildProjects.Verify(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()), Times.AtLeastOnce);
        context.Retention.Verify(x => x.PruneAsync(project.Id, It.IsAny<CancellationToken>()), Times.AtLeastOnce);
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
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<global::Domain.Entities.Activities.ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        unitOfWork.SetupGet(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((global::Domain.Entities.Identity.Actor?)null);
        unitOfWork.SetupGet(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.RollbackAsync())
            .Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static BuildExecutionTestContext CreateExecutionContext(
        BuildProject project,
        GitRepository repository,
        PlatformConnectionInfo platform,
        Registry registry,
        BuildRun run,
        IBuildRunCoordinator? coordinator = null)
    {
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildRunLogs = new Mock<IBuildRunLogRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries);
        var repoCache = new Mock<IRepoCacheManager>(MockBehavior.Strict);
        var runner = new Mock<IBuildProcessRunner>(MockBehavior.Strict);
        var projectStream = new Mock<IBuildProjectStreamManager>(MockBehavior.Strict);
        var runStream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);
        var activityStream = new Mock<IActivityStreamManager>(MockBehavior.Strict);
        var retention = new Mock<IBuildRunRetentionService>(MockBehavior.Strict);
        var secretDefinitions = new Mock<ISecretDefinitionRepository>(MockBehavior.Strict);

        unitOfWork.SetupGet(x => x.BuildRunLogs).Returns(buildRunLogs.Object);
        unitOfWork.SetupGet(x => x.SecretDefinitions).Returns(secretDefinitions.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        buildRuns
            .Setup(x => x.TryClaimAsync(run.Id, It.IsAny<DateTimeOffset>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        gitRepositories
            .Setup(x => x.GetWithAccountAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        platforms
            .Setup(x => x.GetInfoAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        registries
            .Setup(x => x.GetAsync(registry.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildProjects
            .Setup(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()))
            .Callback<Guid, Guid, CancellationToken>((_, runId, _) => project.ReleaseProcessing(runId))
            .ReturnsAsync(1);
        runStream
            .Setup(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);
        projectStream
            .Setup(x => x.SendBuildProjectInfo(It.IsAny<BuildProject>(), It.IsAny<string>(), It.IsAny<BuildRun?>()))
            .Returns(Task.CompletedTask);
        activityStream
            .Setup(x => x.SendActivityInfo(It.IsAny<ActivityEvent>()))
            .Returns(Task.CompletedTask);
        retention
            .Setup(x => x.PruneAsync(project.Id, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var service = new BuildRunExecutionService(
            unitOfWork.Object,
            repoCache.Object,
            runner.Object,
            coordinator ?? new BuildRunCoordinator(),
            Mock.Of<ISecretValueProtector>(),
            Mock.Of<IExternalSecretProviderClient>(),
            projectStream.Object,
            runStream.Object,
            activityStream.Object,
            retention.Object,
            NullLogger<BuildRunExecutionService>.Instance);

        return new BuildExecutionTestContext(
            unitOfWork,
            buildProjects,
            buildRuns,
            buildRunLogs,
            repoCache,
            runner,
            projectStream,
            runStream,
            retention,
            service);
    }

    private static void CaptureLogs(Mock<IBuildRunLogRepository> buildRunLogs, List<BuildRunLogEntry> persistedLogs)
    {
        buildRunLogs
            .Setup(x => x.AddAsync(It.IsAny<BuildRunLogEntry>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRunLogEntry, CancellationToken>((entry, _) => persistedLogs.Add(entry))
            .ReturnsAsync(1);
        buildRunLogs
            .Setup(x => x.AddRangeAsync(It.IsAny<IReadOnlyCollection<BuildRunLogEntry>>(), It.IsAny<CancellationToken>()))
            .Callback<IReadOnlyCollection<BuildRunLogEntry>, CancellationToken>((entries, _) => persistedLogs.AddRange(entries))
            .ReturnsAsync(1);
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
            null,
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

    private static BuildRun CreateRun(
        BuildProject project,
        GitRepository repository,
        PlatformConnectionInfo platform,
        Registry registry,
        Guid actorId,
        Guid? id = null)
    {
        var run = new BuildRun(
            project.Id,
            project.Name,
            repository.Id,
            repository.Name,
            project.Branch,
            null,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [.. project.BuildSecrets.Select(static secret => secret.Id)],
            new BuildPlatformSnapshot(platform.Id, platform.Name, platform.Address, platform.ConnectorType),
            new BuildRegistrySnapshot(registry.Id, registry.Name, registry.RegistryHost),
            project.ImageRepository,
            project.TagTemplates,
            ["registry.example.test/citadel/api:pending"],
            BuildRunTrigger.Manual,
            null,
            actorId,
            project.TimeoutSeconds);

        return id.HasValue
            ? BuildRun.FromPersistence(
                id.Value,
                run.BuildProjectId,
                run.ProjectNameSnapshot,
                run.GitRepositoryId,
                run.GitRepositoryNameSnapshot,
                run.Branch,
                run.ResolvedCommitSha,
                run.ContextPath,
                run.DockerfilePath,
                run.Target,
                run.BuildArgsSnapshot,
                run.BuildSecretIdsSnapshot,
                run.PlatformSnapshot,
                run.RegistrySnapshot,
                run.ImageRepository,
                run.TagTemplatesSnapshot,
                run.ImageReferences,
                run.Trigger,
                run.TriggerSourceId,
                run.Status,
                run.ImageDigest,
                run.TimeoutSeconds,
                run.QueuedAt,
                run.StartedAt,
                run.CompletedAt,
                run.ExitCode,
                run.ErrorCode,
                run.ErrorMessage,
                run.TriggeredByActorId)
            : run;
    }

    private static void PrepareRepositoryCache(GitRepository repository)
    {
        var path = Path.GetFullPath(repository.GetCachePath());
        Directory.CreateDirectory(path);
        File.WriteAllText(Path.Combine(path, "Dockerfile"), "FROM scratch");
    }

    private static async IAsyncEnumerable<BuildProcessEvent> BuildEvents(params BuildProcessEvent[] events)
    {
        await Task.Yield();
        foreach (var item in events)
            yield return item;
    }

    private static async IAsyncEnumerable<BuildProcessEvent> CancellableBuild(
        TaskCompletionSource started,
        TaskCompletionSource cancelled,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        started.SetResult();
        try
        {
            await Task.Delay(TimeSpan.FromSeconds(30), cancellationToken);
        }
        catch (OperationCanceledException)
        {
            cancelled.SetResult();
            throw;
        }
    }

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

    private sealed record BuildExecutionTestContext(
        Mock<IUnitOfWork> UnitOfWork,
        Mock<IBuildProjectRepository> BuildProjects,
        Mock<IBuildRunRepository> BuildRuns,
        Mock<IBuildRunLogRepository> BuildRunLogs,
        Mock<IRepoCacheManager> RepoCache,
        Mock<IBuildProcessRunner> Runner,
        Mock<IBuildProjectStreamManager> ProjectStream,
        Mock<IBuildRunStreamManager> BuildRunStream,
        Mock<IBuildRunRetentionService> Retention,
        BuildRunExecutionService Service);
}
