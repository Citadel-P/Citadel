using Application.Configs;
using Application.Services;
using Application.Services.Abstractions;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class PlatformsStatsWriterJobTests : IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> platformFactoryMock = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();

    private readonly Mock<ISignalRConnectionTracker> connectionTrackerMock = new();
    private readonly Mock<IOptions<JobConfiguration>> configMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly Mock<IPlatformHubDispatcher> hubMock = new();

    private Guid platformId;
    private const int batchSize = 2;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();

        services.AddHostedService<PlatformStatsWriterJob>();
        services.AddHostedService<PlatformStatsStreamerJob>();

        services.AddSingleton(_ => hubMock.Object);
        services.AddSingleton(_ => configMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => connectionTrackerMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(_ => broadcaster);

        configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = batchSize
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task WritesPlatformStatsToDatabase_WhenReceived()
    {
        // Arrange
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(platformConnector.Object);

        connectionTrackerMock.Setup(x => x.HasUsersInGroup(It.IsAny<string>())).Returns(true);

        platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken); // wait for jobs to process

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(platformId, TestContext.Current.CancellationToken);

        Assert.Equal(batchSize, stats.Count());
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stat1 = new PlatformStatsResult
        (
            memTotal: 123456,
            imageCount: 5,
            volumeCount: 2,
            networkCount: 1,
            containerCount: 3,
            containersPaused: 0,
            containersStopped: 1,
            containersRunning: 2,
            platformStat: new DockerPlatformStat
            (
                created: time,
                memoryUsage: 500,
                cpuUsage: 2,
                rxBytes: 100,
                txBytes: 200
            )
        );

        var stat2 = new PlatformStatsResult
        (
            memTotal: 123456,
            imageCount: 6,
            volumeCount: 3,
            networkCount: 10,
            containerCount: 5,
            containersPaused: 1,
            containersStopped: 1,
            containersRunning: 3,
            platformStat: new DockerPlatformStat
            (
                created: time + (60 * 2),
                memoryUsage: 800,
                cpuUsage: 2,
                rxBytes: 100,
                txBytes: 200
            )
        );
        yield return stat1;
        await Task.Delay(100);
        yield return stat2;
        await Task.CompletedTask;
    }

    [Fact]
    public async Task ExecuteAsync_ShouldFlushWhenFlushIntervalIsReached()
    {
        // Arrange
        configMock.Setup(x => x.Value).Returns(new JobConfiguration { BatchSize = 100, FlashInterval = 1 });
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(platformConnector.Object);
        connectionTrackerMock.Setup(x => x.HasUsersInGroup(It.IsAny<string>())).Returns(true);
        platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(15000, TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var db = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stats = await db.PlatformStats.GetStatsAggregatedLast24HoursAsync(platformId, TestContext.Current.CancellationToken);

        Assert.Equal(2, stats.Count());
    }

    [Fact]
    public async Task ExecuteAsync_ShouldNotNotifyClients_WhenNoUsersInGroup()
    {
        // Arrange
        configMock.Setup(x => x.Value).Returns(new JobConfiguration { BatchSize = 1, FlashInterval = 60 });
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(platformConnector.Object);
        connectionTrackerMock.Setup(x => x.HasUsersInGroup(It.IsAny<string>())).Returns(false);
        platformConnector.Setup(x => x.StreamStatsAsync(It.IsAny<StreamPlatformStatsCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StreamPlatformStatsCommand _, CancellationToken __) => GetStatsAsync());

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(100, TestContext.Current.CancellationToken);

        // Assert
        hubMock.Verify(h => h.PushPlatformStats(It.IsAny<Guid>(), It.IsAny<PlatformStatsResult>()), Times.Never);
    }
}
