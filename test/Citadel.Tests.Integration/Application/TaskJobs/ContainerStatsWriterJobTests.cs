using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using System.Collections.Immutable;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ContainerStatsWriterJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IContainerStreamManager> _containerStreamManagerMock = new();
    private readonly Channel<ContainersStatBatch> _channel = Channel.CreateUnbounded<ContainersStatBatch>();

    private readonly Mock<IConnectorFactory<IContainerConnector>> _containerFactoryMock = new();
    private readonly Mock<IContainerConnector> _containerConnectorMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();
    private readonly ObservableDbWorkQueue _dbWorkQueue = new();

    private Guid _platformId;
    private Guid _containerId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services
            .AddHostedService<DbWriteWorker>()
            .AddHostedService<NotificationWorker>()
            .AddHostedService<ContainerStatsWriterJob>();

        services.AddSingleton(_configMock.Object);
        services.AddSingleton(_containerFactoryMock.Object);
        services.AddSingleton(_containerConnectorMock.Object);
        services.AddSingleton(_containerStreamManagerMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_broadcaster);
        services.AddSingleton<IContainerStatsBroadcaster, ContainerStatsBroadcaster>();
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(_dbWorkQueue);
        services.AddSingleton(_channel);
        services.AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<ContainersStatBatch>>().Writer);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration
        {
            BatchSize = 2,
            FlashInterval = 1
        });
        _containerStreamManagerMock
            .Setup(x => x.HasStatsSubscribers(It.IsAny<Guid>()))
            .Returns(true);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var container = new Container(
                name: "container-1",
                dockerImageId: "image-id-1",
                platformId: platform.Id,
                ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
                dockerContainerId: "container-id-1",
                state: ContainerStateStatus.Running);
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        _platformId = platform.Id;
        _containerId = container.Id;

        var cache = Services.GetRequiredService<IPlatformContainerCache>();
        cache.ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(
                Id: platform.Id,
                Address: platform.Address,
                ConnectorType: platform.ConnectorType,
                Containers: new Dictionary<string, Guid>
                {
                    [container.DockerContainerId] = container.Id
                }.ToImmutableDictionary()));
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenBatchSizeIsReached()
    {
        // Arrange
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stats = new List<ContainerStat>();
        stats.AddRange(
        [
            new(_containerId, 100, 200, 5, 300, 100, 200, time),
            new(_containerId, 200, 150, 2, 600, 200, 400, time - 60),
        ]);

        var batch = new ContainersStatBatch(_platformId, stats, (e) => { });

        // Act
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        _containerStreamManagerMock.Verify(d => d.SendContainersStats(_platformId, It.IsAny<IEnumerable<ContainerStat>>()), Times.Once);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync("container-id-1", TestContext.Current.CancellationToken);
        Assert.Equal(2, containers.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenFlushIntervalIsReached()
    {
        // Arrange
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var batch = new ContainersStatBatch(
            _platformId,
            [new ContainerStat(_containerId, 100, 200, 5, 300, 100, 200, time)],
            _ => { });

        // Act
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        _containerStreamManagerMock.Verify(d => d.SendContainersStats(_platformId, It.IsAny<IEnumerable<ContainerStat>>()), Times.Once);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync("container-id-1", TestContext.Current.CancellationToken);
        Assert.Single(containers);
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushPartialBatchWhileInputIsIdle()
    {
        _configMock.Setup(c => c.Value).Returns(new JobConfiguration
        {
            BatchSize = 100,
            FlashInterval = 1
        });
        var batch = new ContainersStatBatch(
            _platformId,
            [
                new ContainerStat(
                    _containerId,
                    100,
                    200,
                    5,
                    300,
                    100,
                    200,
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds())
            ],
            _ => { });

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync(
            "container-id-1",
            TestContext.Current.CancellationToken);

        Assert.Single(stats);
    }

    [Fact]
    public async Task ExecuteAsync_ShouldReleaseBatchWithoutAllocatingNotificationWhenThereAreNoSubscribers()
    {
        _configMock.Setup(c => c.Value).Returns(new JobConfiguration { BatchSize = 100, FlashInterval = 300 });
        _containerStreamManagerMock
            .Setup(x => x.HasStatsSubscribers(It.IsAny<Guid>()))
            .Returns(false);
        var released = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var batch = new ContainersStatBatch(
            _platformId,
            [new ContainerStat(_containerId, 100, 200, 5, 300, 100, 200, 1)],
            _ => released.TrySetResult());

        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await released.Task.WaitAsync(TimeSpan.FromSeconds(2), TestContext.Current.CancellationToken);

        _containerStreamManagerMock.Verify(
            manager => manager.SendContainersStats(
                It.IsAny<Guid>(),
                It.IsAny<IEnumerable<ContainerStat>>()),
            Times.Never);
    }

    [Fact]
    public async Task ExecuteAsync_ShouldDropStatsWhenTheContainerIsNotInThePlatformSnapshot()
    {
        _configMock.Setup(c => c.Value).Returns(new JobConfiguration { BatchSize = 1, FlashInterval = 300 });
        var batch = new ContainersStatBatch(
            _platformId,
            [new ContainerStat(Guid.CreateVersion7(), 100, 200, 5, 300, 100, 200, DateTimeOffset.UtcNow.ToUnixTimeSeconds())],
            _ => { });

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync(
            "container-id-1",
            TestContext.Current.CancellationToken);
        Assert.Empty(stats);
    }

    [Fact]
    public async Task ExecuteAsync_ShouldDropNodeStatsWhenPersistedContainerBelongsToAnotherDataSource()
    {
        _configMock.Setup(c => c.Value).Returns(new JobConfiguration { BatchSize = 1, FlashInterval = 300 });
        var batch = new ContainersStatBatch(
            _platformId,
            [new ContainerStat(_containerId, 100, 200, 5, 300, 100, 200, DateTimeOffset.UtcNow.ToUnixTimeSeconds())],
            _ => { },
            "worker-node");

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(batch, TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.ContainerStats.GetStatsAggregatedLast24HoursAsync(
            "container-id-1",
            TestContext.Current.CancellationToken);
        Assert.Empty(stats);
    }

    [Fact]
    public async Task ExecuteAsync_ShouldPersistSwarmTaskStatsAfterItsTaskContainerIsDeleted()
    {
        var observedAt = DateTimeOffset.UtcNow;
        var taskContainer = new Container(
            name: "redis.1.current",
            dockerImageId: "sha256:redis",
            platformId: _platformId,
            dockerContainerId: "swarm-container-current",
            state: ContainerStateStatus.Running,
            isSwarmTask: true,
            dockerNodeId: "worker-node",
            projectionObservedAt: observedAt.ToUnixTimeSeconds());
        var service = new SwarmServiceProjection(
            _platformId,
            "docker-service",
            1,
            "redis",
            "Replicated",
            "redis:latest",
            1,
            1,
            "Completed",
            null,
            [],
            [],
            [],
            [],
            new Dictionary<string, string>(),
            observedAt,
            observedAt,
            observedAt,
            false,
            SwarmServiceOwnership.Unmanaged);
        var task = new SwarmTaskProjection(
            _platformId,
            "docker-task-current",
            1,
            "redis.1.current",
            service.DockerServiceId,
            service.Name,
            1,
            "worker-node",
            "worker",
            "Running",
            "Running",
            null,
            null,
            service.Image,
            [],
            observedAt,
            observedAt,
            observedAt,
            observedAt,
            false,
            taskContainer.DockerContainerId);

        await using (var scope = Services.CreateAsyncScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await db.Containers.AddAsync(taskContainer, TestContext.Current.CancellationToken);
            await db.Swarm.ReplaceAsync(
                _platformId,
                new SwarmProjectionSnapshot([], [service], [task], [], [], []),
                TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            _platformId,
            new PlatformCacheEntry(
                _platformId,
                "https://original.address",
                PlatformConnectorType.Agent,
                new Dictionary<string, Guid>
                {
                    ["container-id-1"] = _containerId,
                    [taskContainer.DockerContainerId] = taskContainer.Id
                }.ToImmutableDictionary()));

        var created = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(
            new ContainersStatBatch(
                _platformId,
                [new ContainerStat(taskContainer.Id, 256, 32, 12.5, 1024, 2048, 1024, created)],
                _ => { },
                "worker-node"),
            TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(checkpoint, TestContext.Current.CancellationToken);

        await using var assertionScope = Services.CreateAsyncScope();
        var assertionDb = assertionScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await assertionDb.SwarmServiceStats.GetStatsAggregatedAsync(
            new SwarmServiceStatIdentity(_platformId, service.DockerServiceId, null, null, service.Name),
            24,
            TestContext.Current.CancellationToken);

        var sample = Assert.Single(stats);
        Assert.Equal(12.5, sample.CpuUsage);
        Assert.Equal(256, sample.MemoryActive);

        Assert.Equal(
            1,
            await assertionDb.Containers.DeleteAsync(
                [taskContainer.Id],
                TestContext.Current.CancellationToken));
        await assertionDb.CommitAsync(TestContext.Current.CancellationToken);

        var retainedStats = await assertionDb.SwarmServiceStats.GetStatsAggregatedAsync(
            new SwarmServiceStatIdentity(_platformId, service.DockerServiceId, null, null, service.Name),
            24,
            TestContext.Current.CancellationToken);
        Assert.Single(retainedStats);
    }

    [Fact]
    public async Task GetStatsAggregatedAsync_ShouldKeepManagedHistoryAcrossTaskReplacementWithoutDoubleCountingSlot()
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var managedServiceId = Guid.CreateVersion7();
        var bucketStart = DateTimeOffset.UtcNow.ToUnixTimeSeconds() / 60 * 60;
        await db.SwarmServiceStats.BulkInsertAsync(
            [
                new SwarmServiceStat(
                    _platformId, "old-docker-service", managedServiceId, null, "redis", "slot:1",
                    "retired-task", 100, 10, 10, 1000, 10, 20, bucketStart + 1),
                new SwarmServiceStat(
                    _platformId, "new-docker-service", managedServiceId, null, "redis", "slot:1",
                    "replacement-task", 300, 30, 30, 1000, 30, 40, bucketStart + 2),
                new SwarmServiceStat(
                    _platformId, "new-docker-service", managedServiceId, null, "redis", "slot:2",
                    "second-slot-task", 50, 5, 5, 1000, 5, 10, bucketStart + 3)
            ],
            TestContext.Current.CancellationToken);
        await db.CommitAsync(TestContext.Current.CancellationToken);

        var stats = await db.SwarmServiceStats.GetStatsAggregatedAsync(
            new SwarmServiceStatIdentity(
                _platformId,
                "new-docker-service",
                managedServiceId,
                null,
                "redis"),
            24,
            TestContext.Current.CancellationToken);

        var sample = Assert.Single(stats);
        Assert.Equal(25, sample.CpuUsage);
        Assert.Equal(250, sample.MemoryActive);
    }

    [Fact]
    public async Task GetStatsAggregatedAsync_ShouldLoadMultipleContainersInOneQuery()
    {
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var secondContainer = new Container(
            name: "container-2",
            dockerImageId: "image-id-2",
            platformId: _platformId,
            ports: new Dictionary<string, IReadOnlyList<HostPortBinding>>(),
            dockerContainerId: "container-id-2",
            state: ContainerStateStatus.Running);
        await db.Containers.AddAsync(secondContainer, TestContext.Current.CancellationToken);
        await db.ContainerStats.BulkInsertAsync(
            [
                new ContainerStat(
                    _containerId,
                    100,
                    200,
                    5,
                    300,
                    100,
                    200,
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds()),
                new ContainerStat(
                    secondContainer.Id,
                    150,
                    250,
                    7,
                    400,
                    150,
                    250,
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds())
            ],
            TestContext.Current.CancellationToken);
        await db.CommitAsync(TestContext.Current.CancellationToken);

        var stats = (await db.ContainerStats.GetStatsAggregatedAsync(
                [_containerId, secondContainer.Id],
                24,
                TestContext.Current.CancellationToken))
            .ToArray();

        Assert.Equal(2, stats.Length);
        Assert.Contains(stats, stat => stat.ContainerId == _containerId && stat.CpuUsage == 5);
        Assert.Contains(stats, stat => stat.ContainerId == secondContainer.Id && stat.CpuUsage == 7);
    }
    
}
