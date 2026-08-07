using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities;
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
    
}
