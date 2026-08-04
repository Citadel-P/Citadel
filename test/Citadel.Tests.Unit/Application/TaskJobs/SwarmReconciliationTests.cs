using Application.Services.Abstractions;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class SwarmReconciliationTests
{
    [Fact]
    public async Task PartialConnectorFailure_ShouldKeepThePreviousSnapshotAndMarkItStale()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<IReadOnlyList<SwarmServiceResult>>(
                new InternalServerError("Services could not be read.")));
        var repository = CreateStaleRepository(platform.Id);

        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        repository.Verify(
            value => value.MarkStaleAsync(platform.Id, It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task SuccessfulRefresh_ShouldLimitThePersistedActiveTasks()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var tasks = Enumerable.Range(0, 501)
            .Select(index => new SwarmTaskResult(
                $"task-{index}", 1, $"task-{index}", string.Empty, null, string.Empty,
                "Running", "Running", null, null, "nginx:latest", [], null, null, null))
            .ToArray();
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>(tasks));
        SwarmProjectionSnapshot? persisted = null;
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, SwarmProjectionSnapshot, CancellationToken>((_, snapshot, _) => persisted = snapshot)
            .ReturnsAsync(501);

        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        Assert.NotNull(persisted);
        Assert.Equal(500, persisted.Tasks.Count);
    }

    [Fact]
    public async Task NotificationFailure_ShouldNotMarkCommittedInventoryStale()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        var notifications = new Mock<INotificationQueue>();
        notifications
            .Setup(value => value.EnqueueAsync(
                It.IsAny<INotificationWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(new ValueTask(Task.FromException(new InvalidOperationException("queue unavailable"))));

        await using var provider = CreateProvider(platform, repository.Object, notifications.Object);
        var job = CreateJob(provider, connector.Object);

        var result = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        repository.Verify(
            value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.MarkStaleAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task DifferentPlatforms_ShouldRefreshConcurrently()
    {
        var first = CreatePlatform();
        Platform second;
        do
        {
            second = CreatePlatform();
        }
        while ((uint)first.Id.GetHashCode() % 64 == (uint)second.Id.GetHashCode() % 64);

        var bothStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var starts = 0;
        var connector = CreateConnector();
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .Returns(async () =>
            {
                if (Interlocked.Increment(ref starts) == 2)
                    bothStarted.TrySetResult();
                await release.Task;
                return Result.Success<IReadOnlyList<SwarmNodeResult>>([]);
            });
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        await using var provider = CreateProvider(
            first,
            repository.Object,
            additionalPlatforms: [second]);
        var job = CreateJob(provider, connector.Object);
        var firstRefresh = job.RefreshAsync(first.Id, TestContext.Current.CancellationToken);
        var secondRefresh = job.RefreshAsync(second.Id, TestContext.Current.CancellationToken);

        try
        {
            await bothStarted.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        }
        finally
        {
            release.TrySetResult();
        }

        var results = await Task.WhenAll(firstRefresh, secondRefresh);
        Assert.All(results, result => Assert.True(result.IsSuccess()));
    }

    [Fact]
    public async Task ConcurrentInitialization_ShouldReadDockerOnlyOnce()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var initialized = false;
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.GetNodesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => initialized ? [CreateNode(platform.Id, isStale: false)] : []);
        repository
            .Setup(value => value.ReplaceAsync(
                platform.Id,
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .Callback(() => initialized = true)
            .ReturnsAsync(0);
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);

        var results = await Task.WhenAll(
            job.EnsureInitializedAsync(platform.Id, TestContext.Current.CancellationToken),
            job.EnsureInitializedAsync(platform.Id, TestContext.Current.CancellationToken));

        Assert.All(results, result => Assert.True(result.IsSuccess()));
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task EnumerationFailure_ShouldBeContainedSoTheNextIterationCanRun()
    {
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .SetupSequence(value => value.GetPlatformsWithLatestStatAsync(It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("database temporarily unavailable"))
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.Setup(value => value.DisposeAsync()).Returns(ValueTask.CompletedTask);
        await using var provider = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .AddSingleton(Mock.Of<IDbWorkQueue>())
            .AddSingleton(Mock.Of<INotificationQueue>())
            .BuildServiceProvider();
        var job = CreateJob(provider, CreateConnector().Object);

        await job.ReconcileOnceAsync(TestContext.Current.CancellationToken);
        await job.ReconcileOnceAsync(TestContext.Current.CancellationToken);

        platforms.Verify(
            value => value.GetPlatformsWithLatestStatAsync(It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task DaemonEventBurst_ShouldScheduleOneAuthoritativeRefresh()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonResourceEventInfo(
            "update",
            ContainerEventType.Service,
            "service-1",
            DaemonEventScope.Swarm);

        for (var index = 0; index < 20; index++)
            await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);

        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task DaemonEventDuringRefresh_ShouldScheduleOneFollowUpRefresh()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var firstRefreshStarted = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var releaseFirstRefresh = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var calls = 0;
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .Returns(async () =>
            {
                if (Interlocked.Increment(ref calls) == 1)
                {
                    firstRefreshStarted.TrySetResult();
                    await releaseFirstRefresh.Task;
                }
                return Result.Success<IReadOnlyList<SwarmNodeResult>>([]);
            });
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonResourceEventInfo(
            "update",
            ContainerEventType.Node,
            "node-1",
            DaemonEventScope.Swarm);

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        var firstBatch = job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        await firstRefreshStarted.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);
        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        releaseFirstRefresh.TrySetResult();
        await firstBatch;
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task ContainerEvent_ShouldRefreshOnlyAfterThePlatformIsKnownToBeSwarm()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        var repository = CreateWritableRepository();
        await using var provider = CreateProvider(platform, repository.Object);
        var job = CreateJob(provider, connector.Object);
        var daemonEvent = new DaemonContainerEventInfo(
            "die",
            "task-container-1",
            null,
            DaemonEventScope.Local);

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var initialization = await job.RefreshAsync(platform.Id, TestContext.Current.CancellationToken);
        Assert.True(initialization.IsSuccess());
        connector.Invocations.Clear();

        await job.NotifyDaemonEventAsync(platform.Id, daemonEvent, TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);
        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task BuilderPruneEvent_ShouldNotRefreshSwarmInventory()
    {
        var platform = CreatePlatform();
        var connector = CreateConnector();
        await using var provider = CreateProvider(platform, CreateWritableRepository().Object);
        var job = CreateJob(provider, connector.Object);

        await job.NotifyDaemonEventAsync(
            platform.Id,
            new DaemonResourceEventInfo(
                "prune",
                ContainerEventType.Builder,
                string.Empty,
                DaemonEventScope.Local),
            TestContext.Current.CancellationToken);
        await job.ReconcilePendingAsync(TestContext.Current.CancellationToken);

        connector.Verify(
            value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task CompleteSnapshot_ShouldReplaceAndCommit()
    {
        var platformId = Guid.CreateVersion7();
        var nodes = new[] { CreateNode(platformId, isStale: false) };
        var snapshot = CreateSnapshot(nodes);
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(platformId, snapshot, It.IsAny<CancellationToken>()))
            .ReturnsAsync(nodes.Length);
        var committed = false;
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository.Object);
        unitOfWork
            .Setup(value => value.CommitAsync(It.IsAny<CancellationToken>()))
            .Callback(() => committed = true)
            .Returns(Task.CompletedTask);
        await new PersistSwarmSnapshotWorkItem(
                platformId,
                snapshot)
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        repository.Verify(
            value => value.ReplaceAsync(platformId, snapshot, It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.MarkStaleAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()),
            Times.Never);
        Assert.True(committed);
        unitOfWork.Verify(value => value.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task FailedSnapshot_ShouldKeepRowsAndMarkThemStale()
    {
        var platformId = Guid.CreateVersion7();
        var staleNodes = new[] { CreateNode(platformId, isStale: true) };
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        repository
            .Setup(value => value.GetNodesAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(staleNodes);
        repository.Setup(value => value.GetServicesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetTasksAsync(platformId, 500, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetNetworksAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetSecretsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetConfigsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository.Object);
        unitOfWork
            .Setup(value => value.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        await new PersistSwarmSnapshotWorkItem(
                platformId,
                snapshot: null)
            .ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        repository.Verify(
            value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>()),
            Times.Once);
        repository.Verify(
            value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        unitOfWork.Verify(value => value.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    private static SwarmProjectionSnapshot CreateSnapshot(IReadOnlyList<SwarmNodeProjection> nodes) =>
        new(nodes, [], [], [], [], []);

    private static Mock<ISwarmConnector> CreateConnector()
    {
        var connector = new Mock<ISwarmConnector>();
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNodeResult>>([]));
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmServiceResult>>([]));
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>([]));
        connector
            .Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>([]));
        connector
            .Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>([]));
        connector
            .Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>([]));
        return connector;
    }

    private static Mock<ISwarmProjectionRepository> CreateStaleRepository(Guid platformId)
    {
        var repository = new Mock<ISwarmProjectionRepository>();
        repository.Setup(value => value.MarkStaleAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync(0);
        repository.Setup(value => value.GetNodesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetServicesAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetTasksAsync(platformId, 500, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetNetworksAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetSecretsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        repository.Setup(value => value.GetConfigsAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        return repository;
    }

    private static Mock<ISwarmProjectionRepository> CreateWritableRepository()
    {
        var repository = new Mock<ISwarmProjectionRepository>();
        repository
            .Setup(value => value.ReplaceAsync(
                It.IsAny<Guid>(),
                It.IsAny<SwarmProjectionSnapshot>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);
        return repository;
    }

    private static ServiceProvider CreateProvider(
        Platform platform,
        ISwarmProjectionRepository repository,
        INotificationQueue? notificationQueue = null,
        IReadOnlyList<Platform>? additionalPlatforms = null)
    {
        var allPlatforms = additionalPlatforms is null
            ? [platform]
            : additionalPlatforms.Prepend(platform).ToArray();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(value => value.GetByIdAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((Guid id, CancellationToken _) => allPlatforms.SingleOrDefault(value => value.Id == id));
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(repository);
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        unitOfWork.Setup(value => value.DisposeAsync()).Returns(ValueTask.CompletedTask);

        return new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .AddSingleton<IDbWorkQueue>(new InlineDbWorkQueue(unitOfWork.Object))
            .AddSingleton(notificationQueue ?? Mock.Of<INotificationQueue>(queue =>
                queue.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()) == ValueTask.CompletedTask))
            .BuildServiceProvider();
    }

    private static SwarmReconciliationJob CreateJob(ServiceProvider provider, ISwarmConnector connector)
    {
        var connectorFactory = new Mock<IConnectorFactory<ISwarmConnector>>();
        connectorFactory.Setup(value => value.GetConnector(PlatformConnectorType.Local)).Returns(connector);
        return new SwarmReconciliationJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            connectorFactory.Object,
            provider.GetRequiredService<IDbWorkQueue>(),
            provider.GetRequiredService<INotificationQueue>(),
            Mock.Of<IApplicationHubDispatcher>(),
            TimeProvider.System,
            Mock.Of<ILogger<SwarmReconciliationJob>>());
    }

    private static Platform CreatePlatform() => new(
        "swarm",
        "unix:///var/run/docker.sock",
        0,
        0,
        0,
        1,
        1024,
        "28.0",
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            "node-1", "10.0.0.1", "Active", true, 1, 1, "daemon-1", 0, 0, 0, 0));

    private static SwarmNodeProjection CreateNode(Guid platformId, bool isStale) =>
        new(
            platformId,
            "node-1",
            1,
            "manager-1",
            "Manager",
            true,
            "Reachable",
            "Ready",
            null,
            "Active",
            "28.0",
            "linux",
            "x86_64",
            "10.0.0.1",
            new Dictionary<string, string>(),
            1,
            1,
            null,
            null,
            DateTimeOffset.UtcNow,
            isStale);

    private sealed class InlineDbWorkQueue(IUnitOfWork unitOfWork) : IDbWorkQueue
    {
        public System.Threading.Channels.ChannelReader<IDbWorkItem> Reader =>
            throw new NotSupportedException();

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken) =>
            EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken) =>
            await item.ExecuteAsync(unitOfWork, cancellationToken);
    }
}
