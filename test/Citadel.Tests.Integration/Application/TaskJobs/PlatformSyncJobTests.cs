using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Text.Json;
using System.Text.Json.Serialization;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class PlatformSyncJobTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> connectorMock = new();
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();
    private readonly Mock<IPlatformStreamManager> hubManagerMock = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly ObservableDbWorkQueue dbWorkQueue = new();
    private string? platformName;
    private Guid platformId;
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services to ensure only PlatformSyncJob handles platform health events
        services.RemoveAll<IHostedService>();
        services.AddHostedService<PlatformSyncJob>();
        services.AddHostedService<DbWriteWorker>();
        services.AddHostedService<NotificationWorker>();
        services.AddSingleton(_ => hubManagerMock.Object);
        services.AddSingleton(_ => connectorMock.Object);
        services.AddSingleton(_ => platformConnector.Object);
        services.AddSingleton(_ => healthMonitorMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>((_) => broadcaster);
        services.RemoveAll<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue>(dbWorkQueue);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platformDescriptor = new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 2,
            ContainersStopped: 1);

        var platform = new Platform(
            name: "Docker-P-01",
            address: "https://original.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: platformDescriptor
        );
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformName = platform.Name;
        platformId = platform.Id;
    }


    [Fact]
    public async Task DockerPlatform_Goes_Online_Should_Update_Platform_Info()
    {
        // Arrange
        connectorMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                      .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(Fakes.GetDummyPlatformResult()));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByNameAsync(platformName ?? "", TestContext.Current.CancellationToken);

        hubManagerMock.Verify(x => x.PushPlatformUpdate(It.IsAny<Platform>()), Times.Once);
        await Verify(platform);
    }

    [Fact]
    public async Task DockerPlatform_Goes_Offline_Should_Update_Platform_Info()
    {
        // Arrange
        connectorMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                      .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(Fakes.GetDummyPlatformResult()));

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: false),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByNameAsync(platformName ?? "", TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            platformId,
            ActivityResourceType.Platform,
            ActivityEventType.PlatformDisconnected,
            1,
            10,
            TestContext.Current.CancellationToken);

        var options = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.Never // Always include all properties, even if default
        };
        var activitySummary = Assert.Single(activities.Items);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
        var disconnected = Assert.IsType<PlatformDisconnected>(activity?.Info);
        Assert.Equal(PlatformStatus.Online, disconnected.PreviousStatus);
        Assert.Equal(PlatformStatus.Offline, disconnected.Platform.Status);
        hubManagerMock.Verify(x => x.PushPlatformUpdate(It.IsAny<Platform>()), Times.Once);
        await Verify(platform);
    }

    [Fact]
    public async Task DockerPlatform_Comes_Back_Online_Should_Add_Connected_Activity()
    {
        // Arrange
        connectorMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                      .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(Fakes.GetDummyPlatformResult()));

        await using (var setupScope = Services.CreateAsyncScope())
        {
            var setupUow = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var setupPlatform = await setupUow.Platforms.GetByIdAsync(platformId, TestContext.Current.CancellationToken);
            setupPlatform?.PartialUpdate(platformStatus: PlatformStatus.Offline);
            await setupUow.Platforms.UpdateAsync(setupPlatform!, TestContext.Current.CancellationToken);
            await setupUow.CommitAsync(TestContext.Current.CancellationToken);
        }

        // Act
        var checkpoint = dbWorkQueue.CreateCheckpoint();
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await dbWorkQueue.WaitForIdleAfterAsync(
            checkpoint,
            TestContext.Current.CancellationToken);

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByNameAsync(platformName ?? "", TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            platformId,
            ActivityResourceType.Platform,
            ActivityEventType.PlatformConnected,
            1,
            10,
            TestContext.Current.CancellationToken);

        var activitySummary = Assert.Single(activities.Items);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
        var connected = Assert.IsType<PlatformConnected>(activity?.Info);
        Assert.Equal(PlatformStatus.Offline, connected.PreviousStatus);
        Assert.Equal(PlatformStatus.Online, connected.Platform.Status);
        Assert.Equal(PlatformStatus.Online, platform?.Status);
    }

    [Fact]
    public async Task PlatformHealth_For_Nonexistent_Platform_Should_Not_Update_Or_Notify()
    {
        // Arrange: Use a random Guid not in the DB
        var nonExistentPlatformId = Guid.NewGuid();

        // Act
        var job = Services.GetServices<IHostedService>().OfType<PlatformSyncJob>().Single();
        await job.SyncPlatform(
            new PlatformHealth(nonExistentPlatformId, "https://notfound.address", PlatformConnectorType.Agent, IsOnLine: true),
            TestContext.Current.CancellationToken);

        // Assert: No hub notification
        hubManagerMock.Verify(x => x.PushPlatformUpdate(It.IsAny<Platform>()), Times.Never);
    }
}
