using Application.Features.Builds.Commands;
using Application.Features.Builds.Models;
using Application.Permissions;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using System.Runtime.CompilerServices;
using Tests.Common;

namespace Tests.Unit.Application.Features.Builds;

public sealed class BuildRunStartTests : IDisposable
{
    private readonly string repositoryCacheRoot = Path.Combine(
        Path.GetTempPath(),
        "citadel-tests",
        "build-runs",
        Guid.NewGuid().ToString("N"));

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
        var deployments = new Mock<IDeploymentRepository>(MockBehavior.Strict);
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
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

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
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

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
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

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
    public async Task QueueBuildRun_ShouldCreateRunForSelfManagedBuildPool()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var project = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries, buildAgentPools);
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
        buildAgentPools
            .Setup(x => x.GetAsync(pool.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(pool);
        buildRuns
            .Setup(x => x.CountActiveByBuildAgentPoolAsync(pool.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        registries
            .Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildProjects
            .Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .Callback<Guid, Guid, CancellationToken>((_, runId, _) => project.MarkProcessing(runId, DateTimeOffset.UtcNow))
            .ReturnsAsync(1);
        buildRuns
            .Setup(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((run, _) => capturedRun = run)
            .ReturnsAsync(1);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
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
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(capturedRun);
        Assert.Equal(pool.Id, capturedRun.PlatformSnapshot.Id);
        Assert.Equal("grpc://builder.example.test:5001", capturedRun.PlatformSnapshot.Address);
        Assert.Equal(PlatformConnectorType.Agent, capturedRun.PlatformSnapshot.ConnectorType);
        platforms.Verify(x => x.GetInfoAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task QueueBuildRun_ShouldCreateRunForEdgeSelfManagedBuildPool()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(
            actorId,
            new SelfManagedVmBuildAgentPoolProviderSpec(
                null,
                CpuArchitecture.Amd64,
                1,
                ConnectionMode: BuildAgentPoolConnectionMode.EdgeAgent));
        var project = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries, buildAgentPools);
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
        buildAgentPools
            .Setup(x => x.GetAsync(pool.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(pool);
        buildRuns
            .Setup(x => x.CountActiveByBuildAgentPoolAsync(pool.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        registries
            .Setup(x => x.GetAsync(project.RegistryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildProjects
            .Setup(x => x.MarkProcessingAsync(project.Id, It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .Callback<Guid, Guid, CancellationToken>((_, runId, _) => project.MarkProcessing(runId, DateTimeOffset.UtcNow))
            .ReturnsAsync(1);
        buildRuns
            .Setup(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .Callback<BuildRun, CancellationToken>((run, _) => capturedRun = run)
            .ReturnsAsync(1);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
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
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        Assert.NotNull(capturedRun);
        Assert.Equal(pool.Id, capturedRun.PlatformSnapshot.Id);
        Assert.Equal($"edge-build-pool://{pool.Id:D}", capturedRun.PlatformSnapshot.Address);
        Assert.Equal(PlatformConnectorType.EdgeAgent, capturedRun.PlatformSnapshot.ConnectorType);
        platforms.Verify(x => x.GetInfoAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task QueueBuildRun_ShouldRejectSelfManagedBuildPoolWhenNoBuilderIsAvailable()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var project = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries, buildAgentPools);
        var handler = new QueueBuildRunHandler(
            unitOfWork.Object,
            CreateUserContextAccessor(actorId),
            Mock.Of<IBuildProjectStreamManager>(),
            Mock.Of<IBuildRunStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new PermissiveLicenseEntitlementService());

        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.HasActiveRunAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        gitRepositories
            .Setup(x => x.GetAsync(project.GitRepositoryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        buildAgentPools
            .Setup(x => x.GetAsync(pool.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(pool);
        buildRuns
            .Setup(x => x.CountActiveByBuildAgentPoolAsync(pool.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(pool.MaxActiveBuilders);

        var result = await handler.Handle(
            new QueueBuildRun(project.Id, new QueueBuildRunInputModel(BuildRunTrigger.Manual, null)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("no available builders", error.Message);
        buildRuns.Verify(x => x.AddAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()), Times.Never);
        buildProjects.Verify(x => x.MarkProcessingAsync(It.IsAny<Guid>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        registries.Verify(x => x.GetAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
        platforms.Verify(x => x.GetInfoAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldNotStartProcess_WhenRunWasAlreadyClaimed()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
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
            Mock.Of<IDeploymentStreamManager>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IApplyDeploymentService>(),
            Mock.Of<IApplyStackService>(),
            Mock.Of<IAlertService>(),
            Mock.Of<IBuildRunRetentionService>(),
            new PermissiveLicenseEntitlementService(),
            NullLogger<BuildRunExecutionService>.Instance);
        var runId = run.Id;

        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns.Object);
        unitOfWork.SetupGet(x => x.BuildRunLogs).Returns(buildRunLogs.Object);
        unitOfWork.SetupGet(x => x.BuildProjects).Returns(buildProjects.Object);
        buildRuns
            .Setup(x => x.GetAsync(runId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        buildRuns
            .Setup(x => x.TryClaimAsync(runId, It.IsAny<DateTimeOffset>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((BuildRun?)null);

        var result = await service.ExecuteAsync(runId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        runner.Verify(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        buildRunLogs.Verify(x => x.AddAsync(It.IsAny<BuildRunLogEntry>(), It.IsAny<CancellationToken>()), Times.Never);
        buildRuns.Verify(
            x => x.TryClaimAsync(runId, It.IsAny<DateTimeOffset>(), It.IsAny<CancellationToken>()),
            Times.Once);
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
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        BuildProcessCommand? processCommand = null;
        var persistedLogs = new List<BuildRunLogEntry>();
        var statusUpdates = new List<BuildRunStatus>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
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
        Assert.Equal(repositoryCachePath, processCommand.WorkingDirectory);
        Assert.Equal(repositoryCachePath, Path.GetFullPath(processCommand.ContextPath));
        Assert.Equal(Path.Combine(repositoryCachePath, "Dockerfile"), processCommand.DockerfilePath);
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
    public async Task ExecuteQueuedBuildRun_ShouldUseSelfManagedBuildPoolAgentTarget()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var project = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = new PlatformConnectionInfo(pool.Id, pool.Name, "grpc://builder.example.test:5001", PlatformConnectorType.Agent);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run, buildAgentPool: pool);
        BuildProcessCommand? processCommand = null;
        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildProcessCommand, CancellationToken>((command, _) => processCommand = command)
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        CaptureLogs(context.BuildRunLogs, []);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(processCommand);
        Assert.Equal("grpc://builder.example.test:5001", processCommand.PlatformAddress);
        Assert.Equal(PlatformConnectorType.Agent, processCommand.PlatformConnectorType);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldResolveDockerfileRelativeToContext_WhenRepoRootPathDoesNotExist()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId, contextPath: "vote", dockerfilePath: "Dockerfile");
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryRoot = PrepareRepositoryCache(repository);
        File.Delete(Path.Combine(repositoryRoot, "Dockerfile"));
        Directory.CreateDirectory(Path.Combine(repositoryRoot, "vote"));
        File.WriteAllText(Path.Combine(repositoryRoot, "vote", "Dockerfile"), "FROM scratch");

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        BuildProcessCommand? processCommand = null;
        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryRoot));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildProcessCommand, CancellationToken>((command, _) => processCommand = command)
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.AlertService
            .Setup(x => x.ProcessAsync(
                AlertType.BuildRunFailed,
                It.Is<AlertEvaluationContext>(alertContext =>
                    alertContext.BuildRunFailures != null
                    && alertContext.BuildRunFailures.Count == 1
                    && alertContext.BuildRunFailures.Single().Id == project.Id
                    && alertContext.BuildRunFailures.Single().RunId == run.Id),
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        CaptureLogs(context.BuildRunLogs, []);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(processCommand);
        Assert.Equal(Path.Combine(repositoryRoot, "vote"), processCommand.ContextPath);
        Assert.Equal(Path.Combine(repositoryRoot, "vote", "Dockerfile"), processCommand.DockerfilePath);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldPersistPoolRunFailureActivityWithoutPlatform()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var project = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = new PlatformConnectionInfo(pool.Id, pool.Name, "grpc://builder.example.test:5001", PlatformConnectorType.Agent);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run, buildAgentPool: pool);
        var activities = new List<ActivityEvent>();
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((activity, _) => activities.Add(activity))
            .ReturnsAsync(1);
        context.UnitOfWork.SetupGet(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        context.AlertService
            .Setup(x => x.ProcessAsync(
                AlertType.BuildRunFailed,
                It.IsAny<AlertEvaluationContext>(),
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns(BuildEvents(
                new BuildProcessEvent(BuildProcessStream.StdErr, "dockerfile parse error"),
                new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 1)));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        CaptureLogs(context.BuildRunLogs, []);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out _));
        var failedActivity = Assert.Single(activities, activity => activity.EventType == ActivityEventType.BuildRunFailed);
        Assert.Null(failedActivity.PlatformId);
        Assert.All(activities, activity => Assert.Null(activity.PlatformId));
        context.AlertService.Verify(
            x => x.ProcessAsync(
                AlertType.BuildRunFailed,
                It.Is<AlertEvaluationContext>(alertContext =>
                    alertContext.BuildRunFailures != null
                    && alertContext.BuildRunFailures.Count == 1
                    && alertContext.BuildRunFailures.Single().Id == project.Id
                    && alertContext.BuildRunFailures.Single().RunId == run.Id),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldUseQueuedRunTargetSnapshot_WhenProjectBuilderChangesAfterQueue()
    {
        var actorId = Guid.CreateVersion7();
        var pool = CreateSelfManagedPool(actorId);
        var queuedProject = CreateProject(actorId, BuildProjectBuilderKind.BuildAgentPool, buildAgentPoolId: pool.Id);
        var currentProject = BuildProject.FromPersistence(
            queuedProject.Id,
            queuedProject.Name,
            queuedProject.NormalizedName,
            queuedProject.Description,
            queuedProject.Enabled,
            queuedProject.GitRepositoryId,
            queuedProject.Branch,
            queuedProject.ContextPath,
            queuedProject.DockerfilePath,
            queuedProject.Target,
            queuedProject.BuildArgs,
            queuedProject.BuildSecrets,
            Guid.CreateVersion7(),
            queuedProject.RegistryId,
            queuedProject.ImageRepository,
            queuedProject.TagTemplates,
            queuedProject.Webhook,
            queuedProject.TimeoutSeconds,
            queuedProject.RetentionRunCount,
            BuildProjectBuilderKind.Platform,
            buildAgentPoolId: null,
            currentRunId: queuedProject.CurrentRunId,
            queuedProject.ControlState,
            queuedProject.ControlStartedAt,
            queuedProject.CreatedByActorId,
            queuedProject.CreatedAt,
            queuedProject.UpdatedAt,
            queuedProject.ArchivedAt,
            queuedProject.RowVersion);
        var repository = CreateRepository(queuedProject.GitRepositoryId, actorId);
        var platformSnapshot = new PlatformConnectionInfo(pool.Id, pool.Name, "grpc://builder.example.test:5001", PlatformConnectorType.Agent);
        var registry = CreateRegistry(queuedProject.RegistryId, actorId);
        var run = CreateRun(queuedProject, repository, platformSnapshot, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(currentProject, repository, platformSnapshot, registry, run, buildAgentPool: pool);
        BuildProcessCommand? processCommand = null;
        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Callback<BuildProcessCommand, CancellationToken>((command, _) => processCommand = command)
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        CaptureLogs(context.BuildRunLogs, []);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(processCommand);
        Assert.Equal("grpc://builder.example.test:5001", processCommand.PlatformAddress);
        Assert.Equal(PlatformConnectorType.Agent, processCommand.PlatformConnectorType);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldUpdateBuildImageDeploymentConsumers_WhenBuildSucceeds()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var deployment = CreateBuildImageDeployment(Guid.CreateVersion7(), project.PlatformId, project.Id, actorId);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        var persistedLogs = new List<BuildRunLogEntry>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.Deployments
            .Setup(x => x.GetBuildImageConsumersAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([deployment]);
        context.Deployments
            .Setup(x => x.UpdateAsync(deployment, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        var image = Assert.IsType<BuildImage>(deployment.Spec!.Image);
        Assert.Equal("registry.example.test/citadel/api@sha256:abc", image.ResolvedImageReference);
        Assert.Equal("sha256:abc", image.ResolvedDigest);
        Assert.Equal(run.Id, image.ResolvedBuildRunId);
        Assert.Null(image.AppliedBuildRunId);
        Assert.Contains(persistedLogs, log => log.Stream == "system" && log.Message.Contains("Updated build image source for deployment", StringComparison.Ordinal));
        context.Deployments.Verify(x => x.UpdateAsync(deployment, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldUpdateAndRedeployStackBuildImageConsumers_WhenBuildSucceeds()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var stack = CreateBuildImageStack(project.PlatformId, project.Id, actorId, redeployOnBuild: true);
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        var persistedLogs = new List<BuildRunLogEntry>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.Stacks
            .Setup(x => x.GetBuildImageConsumerStacksAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack]);
        context.Stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.ApplyStack
            .Setup(x => x.ApplyAsync(
                stack.Id,
                actorId,
                It.Is<IReadOnlyList<string>>(services => services.Count == 1 && services[0] == "api"),
                true,
                false,
                false,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns((Guid _, Guid _, IReadOnlyList<string>? _, bool _, bool _, bool _, StackApplyOperation _, StackSnapshot? _, CancellationToken ct) => SuccessfulStackApply(ct));
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        var binding = Assert.Single(stack.CurrentStackRelease!.Spec.BuildImageBindings!);
        Assert.Equal("registry.example.test/citadel/api@sha256:abc", binding.ResolvedImageReference);
        Assert.Equal("sha256:abc", binding.ResolvedDigest);
        Assert.Equal(run.Id, binding.ResolvedBuildRunId);
        Assert.Null(binding.AppliedBuildRunId);
        Assert.Contains(persistedLogs, log => log.Stream == "system" && log.Message.Contains("Updated build image binding for stack", StringComparison.Ordinal));
        context.Stacks.Verify(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        context.ApplyStack.Verify(x => x.ApplyAsync(
            stack.Id,
            actorId,
            It.Is<IReadOnlyList<string>>(services => services.Count == 1 && services[0] == "api"),
            true,
            false,
            false,
            StackApplyOperation.Apply,
            null,
            It.IsAny<CancellationToken>()), Times.Once);
        context.StackStream.Verify(x => x.SendStackInfo(stack, "update"), Times.Once);
        context.BuildRunStream.Verify(x => x.SendBuildRunLogs(
            run.Id,
            It.Is<IReadOnlyList<BuildRunLogEntry>>(entries =>
                entries.Any(entry => entry.Message.Contains("Updated build image binding for stack", StringComparison.Ordinal)))),
            Times.Once);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldRemainSucceeded_WhenDeploymentRedeployYieldsError()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var deployment = CreateBuildImageDeployment(Guid.CreateVersion7(), project.PlatformId, project.Id, actorId, redeployOnBuild: true);
        var previousAppliedRunId = Guid.CreateVersion7();
        var previousAppliedAt = DateTimeOffset.UtcNow.AddMinutes(-5);
        var previousBuildImage = Assert.IsType<BuildImage>(deployment.Spec!.Image);
        deployment.PartialUpdate(spec: deployment.Spec with
        {
            Image = previousBuildImage with
            {
                AppliedImageReference = "registry.example.test/citadel/api:previous",
                AppliedDigest = "sha256:previous",
                AppliedBuildRunId = previousAppliedRunId,
                AppliedAt = previousAppliedAt
            }
        });
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        var persistedLogs = new List<BuildRunLogEntry>();
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.Deployments
            .Setup(x => x.GetBuildImageConsumersAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([deployment]);
        context.Deployments
            .Setup(x => x.UpdateAsync(deployment, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.ApplyDeployment
            .Setup(x => x.ApplyAsync(deployment.Id, actorId, false, It.IsAny<CancellationToken>()))
            .Returns((Guid _, Guid _, bool _, CancellationToken ct) => FailedDeploymentApply(ct));

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.Equal(BuildRunStatus.Succeeded, run.Status);
        var image = Assert.IsType<BuildImage>(deployment.Spec!.Image);
        Assert.Equal(run.Id, image.ResolvedBuildRunId);
        Assert.Equal("sha256:abc", image.ResolvedDigest);
        Assert.Equal(previousAppliedRunId, image.AppliedBuildRunId);
        Assert.Equal("sha256:previous", image.AppliedDigest);
        Assert.Equal(previousAppliedAt, image.AppliedAt);
        Assert.Contains(persistedLogs, log => log.Stream == "stderr" && log.Message.Contains("Deployment redeploy failed", StringComparison.Ordinal));
        context.BuildProjects.Verify(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()), Times.Once);
        context.Retention.Verify(x => x.PruneAsync(project.Id, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task ExecuteQueuedBuildRun_ShouldRemainSucceeded_WhenStackRedeployYieldsFailure()
    {
        var actorId = Guid.CreateVersion7();
        var project = CreateProject(actorId);
        var repository = CreateRepository(project.GitRepositoryId, actorId);
        var platform = CreatePlatform(project.PlatformId);
        var registry = CreateRegistry(project.RegistryId, actorId);
        var stack = CreateBuildImageStack(project.PlatformId, project.Id, actorId, redeployOnBuild: true);
        var previousAppliedRunId = Guid.CreateVersion7();
        var previousAppliedAt = DateTimeOffset.UtcNow.AddMinutes(-5);
        var currentRelease = stack.CurrentStackRelease!;
        var previousBinding = Assert.Single(currentRelease.Spec.BuildImageBindings!);
        currentRelease.UpdateSpec(currentRelease.Spec.WithBuildImageBindings(
        [
            previousBinding with
            {
                AppliedImageReference = "registry.example.test/citadel/api:previous",
                AppliedDigest = "sha256:previous",
                AppliedBuildRunId = previousAppliedRunId,
                AppliedAt = previousAppliedAt
            }
        ]));
        var run = CreateRun(project, repository, platform, registry, actorId);
        run.MarkPreparing(DateTimeOffset.UtcNow);
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var context = CreateExecutionContext(project, repository, platform, registry, run);
        var persistedLogs = new List<BuildRunLogEntry>();
        CaptureLogs(context.BuildRunLogs, persistedLogs);

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
        context.Runner
            .Setup(x => x.RunAsync(It.IsAny<BuildProcessCommand>(), It.IsAny<CancellationToken>()))
            .Returns(BuildEvents(new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: "sha256:abc")));
        context.BuildRuns
            .Setup(x => x.UpdateAsync(It.IsAny<BuildRun>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.Stacks
            .Setup(x => x.GetBuildImageConsumerStacksAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack]);
        context.Stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        context.ApplyStack
            .Setup(x => x.ApplyAsync(
                stack.Id,
                actorId,
                It.Is<IReadOnlyList<string>>(services => services.Count == 1 && services[0] == "api"),
                true,
                false,
                false,
                StackApplyOperation.Apply,
                null,
                It.IsAny<CancellationToken>()))
            .Returns((Guid _, Guid _, IReadOnlyList<string>? _, bool _, bool _, bool _, StackApplyOperation _, StackSnapshot? _, CancellationToken ct) => FailedStackApply(ct));

        var result = await context.Service.ExecuteAsync(run.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.Equal(BuildRunStatus.Succeeded, run.Status);
        var binding = Assert.Single(stack.CurrentStackRelease!.Spec.BuildImageBindings!);
        Assert.Equal(run.Id, binding.ResolvedBuildRunId);
        Assert.Equal("sha256:abc", binding.ResolvedDigest);
        Assert.Equal(previousAppliedRunId, binding.AppliedBuildRunId);
        Assert.Equal("sha256:previous", binding.AppliedDigest);
        Assert.Equal(previousAppliedAt, binding.AppliedAt);
        Assert.Contains(
            persistedLogs,
            log => log.Stream == "stderr"
                && log.Message.Contains("Stack redeploy failed", StringComparison.Ordinal)
                && log.Message.Contains("redeploy failed", StringComparison.Ordinal));
        context.BuildProjects.Verify(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()), Times.Once);
        context.Retention.Verify(x => x.PruneAsync(project.Id, It.IsAny<CancellationToken>()), Times.Once);
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
        var repositoryCachePath = PrepareRepositoryCache(repository);

        var coordinator = new BuildRunCoordinator();
        var context = CreateExecutionContext(project, repository, platform, registry, run, coordinator);
        var runnerStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var runnerCancelled = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var statusUpdates = new List<BuildRunStatus>();
        var persistedLogs = new List<BuildRunLogEntry>();

        context.RepoCache
            .Setup(x => x.SynchronizeAsync(repository, repository.GitAccount, "main", It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "abcdef1234567890",
                Success: true,
                CachePath: repositoryCachePath));
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
        var permissionEvaluator = new Mock<IPermissionEvaluator>();
        permissionEvaluator
            .Setup(x => x.EvaluateAsync(
                project.Id,
                ResourceType.Build,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Helpers.AdminPermissions);

        var cancelHandler = new CancelBuildRunHandler(
            context.UnitOfWork.Object,
            permissionEvaluator.Object,
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
        Mock<IRegistryRepository> registries,
        Mock<IBuildAgentPoolRepository>? buildAgentPools = null)
    {
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.BuildProjects).Returns(buildProjects.Object);
        unitOfWork.SetupGet(x => x.BuildRuns).Returns(buildRuns.Object);
        unitOfWork.SetupGet(x => x.GitRepositories).Returns(gitRepositories.Object);
        unitOfWork.SetupGet(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(x => x.Registries).Returns(registries.Object);
        unitOfWork.SetupGet(x => x.BuildAgentPools).Returns(buildAgentPools?.Object ?? Mock.Of<IBuildAgentPoolRepository>());
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
        IBuildRunCoordinator? coordinator = null,
        BuildAgentPool? buildAgentPool = null)
    {
        var buildRuns = new Mock<IBuildRunRepository>(MockBehavior.Strict);
        var buildRunLogs = new Mock<IBuildRunLogRepository>(MockBehavior.Strict);
        var buildProjects = new Mock<IBuildProjectRepository>(MockBehavior.Strict);
        var gitRepositories = new Mock<IGitReposRepository>(MockBehavior.Strict);
        var platforms = new Mock<IPlatformRepository>(MockBehavior.Strict);
        var registries = new Mock<IRegistryRepository>(MockBehavior.Strict);
        var buildAgentPools = new Mock<IBuildAgentPoolRepository>(MockBehavior.Strict);
        var deployments = new Mock<IDeploymentRepository>(MockBehavior.Strict);
        var stacks = new Mock<IStackRepository>(MockBehavior.Strict);
        var unitOfWork = CreateUnitOfWork(buildProjects, buildRuns, gitRepositories, platforms, registries, buildAgentPools);
        var repoCache = new Mock<IRepoCacheManager>(MockBehavior.Strict);
        var runner = new Mock<IBuildProcessRunner>(MockBehavior.Strict);
        var projectStream = new Mock<IBuildProjectStreamManager>(MockBehavior.Strict);
        var runStream = new Mock<IBuildRunStreamManager>(MockBehavior.Strict);
        var activityStream = new Mock<IActivityStreamManager>(MockBehavior.Strict);
        var deploymentStream = new Mock<IDeploymentStreamManager>(MockBehavior.Strict);
        var stackStream = new Mock<IStackStreamManager>(MockBehavior.Strict);
        var applyDeployment = new Mock<IApplyDeploymentService>(MockBehavior.Strict);
        var applyStack = new Mock<IApplyStackService>(MockBehavior.Strict);
        var retention = new Mock<IBuildRunRetentionService>(MockBehavior.Strict);
        var secretDefinitions = new Mock<ISecretDefinitionRepository>(MockBehavior.Strict);
        var alertService = new Mock<IAlertService>(MockBehavior.Strict);

        unitOfWork.SetupGet(x => x.BuildRunLogs).Returns(buildRunLogs.Object);
        unitOfWork.SetupGet(x => x.SecretDefinitions).Returns(secretDefinitions.Object);
        unitOfWork.SetupGet(x => x.Deployments).Returns(deployments.Object);
        unitOfWork.SetupGet(x => x.Stacks).Returns(stacks.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var queuedRun = CreateQueuedRunSnapshot(run);
        buildRuns
            .Setup(x => x.GetAsync(run.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(queuedRun);
        buildRuns
            .Setup(x => x.TryClaimAsync(run.Id, It.IsAny<DateTimeOffset>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), true))
            .ReturnsAsync(project);
        buildProjects
            .Setup(x => x.GetAsync(project.Id, It.IsAny<CancellationToken>(), false))
            .ReturnsAsync(project);
        gitRepositories
            .Setup(x => x.GetWithAccountAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        if (buildAgentPool is not null)
        {
            platforms
                .Setup(x => x.GetInfoAsync(platform.Id, It.IsAny<CancellationToken>()))
                .ReturnsAsync((PlatformConnectionInfo?)null);
            buildAgentPools
                .Setup(x => x.GetAsync(buildAgentPool.Id, It.IsAny<CancellationToken>(), true))
                .ReturnsAsync(buildAgentPool);
        }
        else
        {
            platforms
                .Setup(x => x.GetInfoAsync(platform.Id, It.IsAny<CancellationToken>()))
                .ReturnsAsync(platform);
        }
        registries
            .Setup(x => x.GetAsync(registry.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        buildProjects
            .Setup(x => x.MarkIdleAsync(project.Id, run.Id, It.IsAny<CancellationToken>()))
            .Callback<Guid, Guid, CancellationToken>((_, runId, _) => project.MarkIdle(runId, DateTimeOffset.UtcNow))
            .ReturnsAsync(1);
        deployments
            .Setup(x => x.GetBuildImageConsumersAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        stacks
            .Setup(x => x.GetBuildImageConsumerStacksAsync(project.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        runStream
            .Setup(x => x.SendBuildRunInfo(It.IsAny<BuildRun>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);
        projectStream
            .Setup(x => x.SendBuildProjectInfo(It.IsAny<BuildProject>(), It.IsAny<string>(), It.IsAny<BuildRun?>()))
            .Returns(Task.CompletedTask);
        activityStream
            .Setup(x => x.SendActivityInfo(It.IsAny<ActivityEvent>()))
            .Returns(Task.CompletedTask);
        deploymentStream
            .Setup(x => x.SendDeploymentInfo(It.IsAny<Deployment>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);
        stackStream
            .Setup(x => x.SendStackInfo(It.IsAny<Stack>(), It.IsAny<string>()))
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
            deploymentStream.Object,
            stackStream.Object,
            applyDeployment.Object,
            applyStack.Object,
            alertService.Object,
            retention.Object,
            new PermissiveLicenseEntitlementService(),
            NullLogger<BuildRunExecutionService>.Instance);

        return new BuildExecutionTestContext(
            unitOfWork,
            buildProjects,
            buildRuns,
            buildRunLogs,
            deployments,
            stacks,
            repoCache,
            runner,
            projectStream,
            runStream,
            stackStream,
            applyDeployment,
            applyStack,
            alertService,
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

    private static BuildProject CreateProject(
        Guid actorId,
        BuildProjectBuilderKind builderKind = BuildProjectBuilderKind.Platform,
        Guid? buildAgentPoolId = null,
        string contextPath = ".",
        string dockerfilePath = "Dockerfile")
        => new(
            "api-image",
            null,
            true,
            Guid.CreateVersion7(),
            "main",
            contextPath,
            dockerfilePath,
            null,
            [],
            [],
            builderKind == BuildProjectBuilderKind.Platform ? Guid.CreateVersion7() : Guid.Empty,
            Guid.CreateVersion7(),
            "citadel/api",
            ["{branch}-{shortSha}"],
            null,
            BuildProject.DefaultTimeoutSeconds,
            BuildProject.DefaultRetentionRunCount,
            actorId,
            builderKind,
            buildAgentPoolId);

    private static BuildAgentPool CreateSelfManagedPool(
        Guid actorId,
        SelfManagedVmBuildAgentPoolProviderSpec? providerSpec = null)
        => new(
            "agent-builders",
            null,
            true,
            providerSpec ?? new SelfManagedVmBuildAgentPoolProviderSpec(
                "grpc://builder.example.test:5001",
                CpuArchitecture.Amd64,
                1),
            BuildAgentPool.DefaultMaxActiveBuilders,
            BuildAgentPool.DefaultQueueTimeoutSeconds,
            BuildAgentPool.DefaultProvisioningTimeoutSeconds,
            BuildAgentPool.DefaultRegistrationTimeoutSeconds,
            BuildAgentPool.DefaultHeartbeatTimeoutSeconds,
            BuildAgentPool.DefaultCleanupTimeoutSeconds,
            BuildAgentPool.DefaultMaximumInstanceLifetimeSeconds,
            BuildAgentPool.DefaultFailureRetentionMinutes,
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

    private static Deployment CreateExternalImageDeployment(Guid id, Guid platformId, Guid registryId, Guid actorId)
        => Deployment.FromPersistence(
            id,
            "api",
            platformId,
            rowVersion: 0,
            controlStartedAt: null,
            controlState: ResourceControlState.Idle,
            status: DeploymentStatus.Created,
            createdAt: DateTime.UtcNow,
            createdByActorId: actorId,
            controlTriggeredBy: null,
            spec: new DeploymentSpec(
                new ExternalImage(registryId, "citadel/api:old", "sha256:old"),
                UpdateBehavior.Disabled));

    private static Deployment CreateBuildImageDeployment(
        Guid id,
        Guid platformId,
        Guid buildProjectId,
        Guid actorId,
        bool redeployOnBuild = false)
        => Deployment.FromPersistence(
            id,
            "api",
            platformId,
            rowVersion: 0,
            controlStartedAt: null,
            controlState: ResourceControlState.Idle,
            status: DeploymentStatus.Created,
            createdAt: DateTime.UtcNow,
            createdByActorId: actorId,
            controlTriggeredBy: null,
            spec: new DeploymentSpec(
                new BuildImage(buildProjectId, redeployOnBuild),
                UpdateBehavior.Disabled));

    private static Stack CreateBuildImageStack(Guid platformId, Guid buildProjectId, Guid actorId, bool redeployOnBuild)
        => Stack.Create(
            "api-stack",
            actorId,
            StackSource.WebEditor,
            platformId,
            new ManualStack(
                ComposeFile: """
                             services:
                               api:
                                 image: registry.example.test/citadel/api:old
                             """,
                UpdateBehavior: StackUpdateBehavior.Disabled,
                BuildImageBindings:
                [
                    new StackBuildImageBinding(
                        ServiceName: "api",
                        BuildProjectId: buildProjectId,
                        RedeployOnBuild: redeployOnBuild,
                        ResolvedImageReference: "registry.example.test/citadel/api:old",
                        ResolvedDigest: "sha256:old")
                ]));

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

    private static BuildRun CreateQueuedRunSnapshot(BuildRun run)
        => BuildRun.FromPersistence(
            run.Id,
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
            BuildRunStatus.Queued,
            run.ImageDigest,
            run.TimeoutSeconds,
            run.QueuedAt,
            startedAt: null,
            completedAt: null,
            exitCode: null,
            errorCode: null,
            errorMessage: null,
            triggeredByActorId: run.TriggeredByActorId);

    private string PrepareRepositoryCache(GitRepository repository)
    {
        var path = Path.GetFullPath(Path.Combine(repositoryCacheRoot, repository.Id.ToString("D")));
        Directory.CreateDirectory(path);
        File.WriteAllText(Path.Combine(path, "Dockerfile"), "FROM scratch");
        return path;
    }

    public void Dispose()
    {
        if (Directory.Exists(repositoryCacheRoot))
            Directory.Delete(repositoryCacheRoot, recursive: true);
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

        yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0);
    }

    private static async IAsyncEnumerable<DeploymentStreamItem> FailedDeploymentApply(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Yield();
        cancellationToken.ThrowIfCancellationRequested();
        yield return new DeploymentStreamItem(
            ErrorMessage: "redeploy failed",
            Error: new DeploymentApplyError(500, "redeploy failed"));
    }

    private static async IAsyncEnumerable<StackStreamItem> FailedStackApply(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Yield();
        cancellationToken.ThrowIfCancellationRequested();
        yield return StackStreamItem.FromStdErr("redeploy failed", 1);
    }

    private static async IAsyncEnumerable<StackStreamItem> SuccessfulStackApply(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await Task.Yield();
        cancellationToken.ThrowIfCancellationRequested();
        yield return StackStreamItem.Finished(0);
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
        Mock<IDeploymentRepository> Deployments,
        Mock<IStackRepository> Stacks,
        Mock<IRepoCacheManager> RepoCache,
        Mock<IBuildProcessRunner> Runner,
        Mock<IBuildProjectStreamManager> ProjectStream,
        Mock<IBuildRunStreamManager> BuildRunStream,
        Mock<IStackStreamManager> StackStream,
        Mock<IApplyDeploymentService> ApplyDeployment,
        Mock<IApplyStackService> ApplyStack,
        Mock<IAlertService> AlertService,
        Mock<IBuildRunRetentionService> Retention,
        BuildRunExecutionService Service);
}
