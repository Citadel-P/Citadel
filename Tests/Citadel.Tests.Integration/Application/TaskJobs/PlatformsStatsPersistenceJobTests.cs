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

namespace Tests.Integration.Application.TaskJobs;

public class PlatformsStatsPersistenceJobTests : IntegrationTestBase<WebApi.Program>
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

        services.AddHostedService<PlatformsStatsPersistenceJob>();
        services.AddHostedService<PlatformsStatsCollectorJob>();

        services.AddSingleton(_ => hubMock.Object);
        services.AddSingleton(_ => configMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => connectionTrackerMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>((_) => broadcaster);

        configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = batchSize
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = GetDummyPlatform();

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task PersistsStatsToDatabase_WhenStatsAreReceived()
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
        var t = await db.Platforms.GetPlatformsWithLatestStatAsync(TestContext.Current.CancellationToken);

        Assert.Equal(batchSize, stats.Count());
    }

    private static async IAsyncEnumerable<PlatformStatsResult> GetStatsAsync()
    {
        var time = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var stat1 = new PlatformStatsResult
        (
            MemTotal: 123456,
            ImageCount: 5,
            VolumeCount: 2,
            NetworkCount: 1,
            ContainerCount: 3,
            ContainersPaused: 0,
            ContainersStopped: 1,
            ContainersRunning: 2,
            PlatformStat: new DockerPlatformStat
            (
                Created: time,
                MemoryUsage: 500,
                CpuUsage: 2,
                RxBytes: 100,
                TxBytes: 200
            )
        );

        var stat2 = new PlatformStatsResult
        (
            MemTotal: 123456,
            ImageCount: 6,
            VolumeCount: 3,
            NetworkCount: 10,
            ContainerCount: 5,
            ContainersPaused: 1,
            ContainersStopped: 1,
            ContainersRunning: 3,
            PlatformStat: new DockerPlatformStat
            (
                Created: time + (60 * 2),
                MemoryUsage: 800,
                CpuUsage: 2,
                RxBytes: 100,
                TxBytes: 200
            )
        );
        yield return stat1;
        await Task.Delay(100);
        yield return stat2;
        await Task.CompletedTask;
    }
}
