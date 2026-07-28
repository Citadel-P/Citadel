using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using System.Threading.Channels;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class PlatformsStatsWriterJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> _platformFactoryMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly Mock<IPlatformConnector> _platformConnector = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();
    private readonly Mock<IPlatformStreamManager> _streamManagerMock = new();
    private readonly Channel<(Guid Id, PlatformStatsResult Stats)> _channel = Channel.CreateUnbounded<(Guid Id, PlatformStatsResult Stats)>();
    private readonly ObservableDbWorkQueue _dbWorkQueue = new();


    private Guid _platformId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IOptions<JobConfiguration>>();

        services.AddHostedService<PlatformStatsWriterJob>();
        services.AddHostedService<PlatformStatsStreamerJob>();
        services.AddHostedService<DbWriteWorker>();
        services.AddHostedService<NotificationWorker>();
        services.AddSingleton(_channel);
        services.AddSingleton(_ => _streamManagerMock.Object);
        services.AddSingleton(_ => _configMock.Object);
        services.AddSingleton(_ => _platformConnector.Object);
        services.AddSingleton(_ => _platformFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_ => _broadcaster);
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(_dbWorkQueue);
        services.AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<(Guid Id, PlatformStatsResult Stats)>>().Writer);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration
        {
            BatchSize = 2,
            FlashInterval = 1
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
    }

    [Fact]
    public async Task WritesPlatformStatsToDatabase_WhenReceived()
    {
        // Arrange
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 2 });

        _platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_platformConnector.Object);
        _platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _broadcaster.PublishAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);

        Assert.Equal(2, stats.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenFlushIntervalIsReached()
    {
        // Arrange
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 100, FlashInterval = 1 });
        _platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_platformConnector.Object);
        _platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _broadcaster.PublishAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);

        Assert.Equal(2, stats.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushPartialBatchWhileInputIsIdle()
    {
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration
        {
            BatchSize = 100,
            FlashInterval = 1
        });
        var stat = new PlatformStatsResult(
            MemTotal: 123456,
            ImageCount: 1,
            VolumeCount: 1,
            NetworkCount: 1,
            AgentVersion: "1.0.0",
            ImageUsedBytes: 2048,
            VolumeUsedBytes: 4096,
            PlatformStat: new DockerPlatformStat(
                created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                memoryUsage: 10,
                cpuUsage: 20,
                rxBytes: 30,
                txBytes: 40,
                containerCount: 1,
                containersPaused: 0,
                containersStopped: 0,
                containersRunning: 1));

        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync(
            (_platformId, stat),
            TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(
            _platformId,
            TestContext.Current.CancellationToken);
        var platform = await db.Platforms.GetByIdAsync(_platformId, TestContext.Current.CancellationToken);
        var descriptor = Assert.IsType<DockerPlatformDescriptor>(platform?.PlatformDescriptor);

        Assert.Single(stats);
        Assert.Equal(2048, descriptor.ImageUsedBytes);
        Assert.Equal(4096, descriptor.VolumeUsedBytes);
    }

    [Fact]
    public async Task GetStatsAggregatedAsync_ShouldRespectRequestedWindow()
    {
        // Arrange
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        await using (var setupScope = Services.CreateAsyncScope())
        {
            var db = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await db.PlatformStats.BulkInsertAsync(
                [
                    new PlatformStat(
                        Created: now - (long)TimeSpan.FromHours(47).TotalSeconds,
                        MemoryUsage: 10,
                        CpuUsage: 20,
                        RxBytes: 30,
                        TxBytes: 40,
                        PlatformId: _platformId),
                    new PlatformStat(
                        Created: now - (long)TimeSpan.FromHours(25).TotalSeconds,
                        MemoryUsage: 50,
                        CpuUsage: 60,
                        RxBytes: 70,
                        TxBytes: 80,
                        PlatformId: _platformId),
                    new PlatformStat(
                        Created: now - (long)TimeSpan.FromHours(73).TotalSeconds,
                        MemoryUsage: 90,
                        CpuUsage: 100,
                        RxBytes: 110,
                        TxBytes: 120,
                        PlatformId: _platformId)
                ],
                TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        // Act
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var last24 = await uow.PlatformStats.GetStatsAggregatedAsync(_platformId, 24, TestContext.Current.CancellationToken);
        var last48 = await uow.PlatformStats.GetStatsAggregatedAsync(_platformId, 48, TestContext.Current.CancellationToken);
        var last72 = await uow.PlatformStats.GetStatsAggregatedAsync(_platformId, 72, TestContext.Current.CancellationToken);

        // Assert
        Assert.Empty(last24);
        Assert.Equal(2, last48.Count());
        Assert.Equal(2, last72.Count());
        Assert.All(last48, stat => Assert.Equal(_platformId, stat.PlatformId));
    }

    [Fact]
    public async Task PlatformStatsRepository_ShouldPreserveDiskValuesAndUnavailableSamples()
    {
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        await using (var setupScope = Services.CreateAsyncScope())
        {
            var uow = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.PlatformStats.BulkInsertAsync(
                [
                    new PlatformStat(
                        Created: now - 120,
                        MemoryUsage: 10,
                        CpuUsage: 20,
                        RxBytes: 30,
                        TxBytes: 40,
                        PlatformId: _platformId,
                        DiskUsedBytes: null,
                        DiskTotalBytes: null,
                        DiskUsage: null),
                    new PlatformStat(
                        Created: now,
                        MemoryUsage: 50,
                        CpuUsage: 60,
                        RxBytes: 70,
                        TxBytes: 80,
                        PlatformId: _platformId,
                        DiskUsedBytes: 75,
                        DiskTotalBytes: 100,
                        DiskUsage: 75)
                ],
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = (await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(
            _platformId,
            TestContext.Current.CancellationToken)).ToArray();
        var latestPlatform = await db.Platforms.GetPlatformWithLatestStatAsync(
            _platformId,
            TestContext.Current.CancellationToken);

        Assert.Equal(2, stats.Length);
        Assert.Null(stats[0].DiskUsedBytes);
        Assert.Null(stats[0].DiskTotalBytes);
        Assert.Null(stats[0].DiskUsage);
        Assert.Equal(75, stats[1].DiskUsedBytes);
        Assert.Equal(100, stats[1].DiskTotalBytes);
        Assert.Equal(75, stats[1].DiskUsage);
        var latestStat = Assert.Single(latestPlatform!.Stats);
        Assert.Equal(75, latestStat.DiskUsedBytes);
        Assert.Equal(100, latestStat.DiskTotalBytes);
        Assert.Equal(75, latestStat.DiskUsage);
    }

    [Fact]
    public async Task PersistingStats_Should_Not_Mark_Offline_Platform_Online()
    {
        // Arrange
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration { BatchSize = 1 });

        await using (var setupScope = Services.CreateAsyncScope())
        {
            var db = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = await db.Platforms.GetByIdAsync(_platformId, TestContext.Current.CancellationToken);
            platform?.PartialUpdate(platformStatus: PlatformStatus.Offline);
            await db.Platforms.UpdateAsync(platform!, TestContext.Current.CancellationToken);
            await db.CommitAsync(TestContext.Current.CancellationToken);
        }

        var stat = new PlatformStatsResult
        (
            MemTotal: 123456,
            ImageCount: 7,
            VolumeCount: 2,
            NetworkCount: 1,
            AgentVersion: "1.0",
            PlatformStat: new DockerPlatformStat
            (
                created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                memoryUsage: 500,
                cpuUsage: 2,
                rxBytes: 100,
                txBytes: 200,
                containerCount: 3,
                containersPaused: 0,
                containersStopped: 1,
                containersRunning: 2
            )
        );

        // Act
        var checkpoint = _dbWorkQueue.CreateCheckpoint();
        await _channel.Writer.WriteAsync((_platformId, stat), TestContext.Current.CancellationToken);
        await _dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platformAfterStats = await uow.Platforms.GetByIdAsync(_platformId, TestContext.Current.CancellationToken);
        var stats = await uow.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);

        Assert.Equal(PlatformStatus.Offline, platformAfterStats?.Status);
        Assert.Equal(7, platformAfterStats?.ImageCount);
        Assert.Single(stats);
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        {
            var stat1 = new PlatformStatsResult
            (
                MemTotal: 123456,
                ImageCount: 5,
                VolumeCount: 2,
                NetworkCount: 1,
                AgentVersion: "1.0",
                PlatformStat: new DockerPlatformStat
                (
                    created: time,
                    memoryUsage: 500,
                    cpuUsage: 2,
                    rxBytes: 100,
                    txBytes: 200,
                    containerCount: 3,
                    containersPaused: 0,
                    containersStopped: 1,
                    containersRunning: 2
                )
            );

            yield return stat1;
        }
        await Task.Yield();
        {
            var stat2 = new PlatformStatsResult
            (
                MemTotal: 123456,
                ImageCount: 6,
                VolumeCount: 3,
                NetworkCount: 10,
                AgentVersion: "1.0",
                PlatformStat: new DockerPlatformStat
                (
                    created: time + (60 * 2),
                    memoryUsage: 800,
                    cpuUsage: 2,
                    rxBytes: 300,
                    txBytes: 400,
                    containerCount: 5,
                    containersPaused: 1,
                    containersStopped: 1,
                    containersRunning: 3
                )
            );
            yield return stat2;
        }
        
    }

}
