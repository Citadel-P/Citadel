using System.Threading.Channels;
using Application.Configs;
using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class PlatformsStatsWriterJobTests : IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> _platformFactoryMock = new();
    private readonly Mock<IOptions<JobConfiguration>> _configMock = new();
    private readonly Mock<IPlatformConnector> _platformConnector = new();
    private readonly TestPlatformHealthBroadCaster _broadcaster = new();
    private readonly Mock<IPlatformsStreamManager> _streamManagerMock = new();
    private readonly Channel<(Guid Id, PooledHandle<PlatformStatsResult> Stats)> _channel = Channel.CreateUnbounded<(Guid Id, PooledHandle<PlatformStatsResult> Stats)>();


    private Guid _platformId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IOptions<JobConfiguration>>();

        services.AddHostedService<PlatformStatsWriterJob>();
        services.AddHostedService<PlatformStatsStreamerJob>();
        services.AddSingleton(_channel);
        services.AddSingleton(_ => _streamManagerMock.Object);
        services.AddSingleton(_ => _configMock.Object);
        services.AddSingleton(_ => _platformConnector.Object);
        services.AddSingleton(_ => _platformFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_ => _broadcaster);

        _configMock.Setup(x => x.Value).Returns(new JobConfiguration());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

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
        await _broadcaster.PublishAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(500, TestContext.Current.CancellationToken); // wait for jobs to process

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
        _configMock.Setup(x => x.Value).Returns(new JobConfiguration() { BatchSize = 100, FlashInterval = 0 });
        _platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_platformConnector.Object);
        _platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        await _broadcaster.PublishAsync(new PlatformHealth(_platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(_platformId, TestContext.Current.CancellationToken);

        Assert.Equal(2, stats.Count());
    }

    [Fact]
    public async Task PooledObjects_ShouldBeReturnedToPool()
    {
        // Arrange
        var objectPoolManager = Services.GetRequiredService<IObjectPoolManager>();
        var pooledStat = objectPoolManager.GetPooled<PlatformStatsResult>();
        pooledStat.Value.ReInitialize(memTotal: 123456,
            imageCount: 5,
            volumeCount: 2,
            networkCount: 1,
            platformStat: new DockerPlatformStat
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
            ));


        _configMock.Setup(x => x.Value).Returns(new JobConfiguration { BatchSize = 1, FlashInterval = 1 });
        _platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(_platformConnector.Object);
        
        // Act
        await _channel.Writer.WriteAsync((_platformId, pooledStat), TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert
        var pooledStat2 = objectPoolManager.GetPooled<PlatformStatsResult>();
        Assert.Same(pooledStat2.Value, pooledStat.Value);
        pooledStat2.Dispose();
    }

    private async IAsyncEnumerable<PooledHandle<PlatformStatsResult>> GetStatsAsync()
    {
        var objectPoolManager = Services.GetRequiredService<IObjectPoolManager>();

        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        {
            var stat1 = objectPoolManager.GetPooled<PlatformStatsResult>();
            stat1.Value.ReInitialize
            (
                memTotal: 123456,
                imageCount: 5,
                volumeCount: 2,
                networkCount: 1,
               
                platformStat: new DockerPlatformStat
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
        await Task.Delay(100);
        {
            var stat2 = objectPoolManager.GetPooled<PlatformStatsResult>();
            stat2.Value.ReInitialize
            (
                memTotal: 123456,
                imageCount: 6,
                volumeCount: 3,
                networkCount: 10,
                platformStat: new DockerPlatformStat
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
