using System.Text.Json;
using System.Text.Json.Serialization;
using Citadel.Agent.Common.V1;
using Grpc.Core;
using Domain;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.EntityFramework;
using Application.Services.Abstractions;
using Application.TaskJobs;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Platforms.V1.PlatformService;
using Application.Services;
using Infrastructure.Services;

namespace Tests.Integration.Infrastructure.TaskJobs;

public class PlatformSyncJobTests : IntegrationTestBase<WebApi.Program>
{
    private readonly Mock<IPlatformHubDispatcher> hubMock = new();
    private readonly Mock<IGrpcClientFactory> grpcFactoryMock = new();
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();

    private readonly Mock<PlatformServiceClient> platformClientMock = new();
    private readonly Mock<ContainerServiceClient> containerClientMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services to ensure only PlatformSyncJob handles platform health events
        services.RemoveAll<IHostedService>();
        services.AddHostedService<PlatformSyncJob>();
        services.AddSingleton(_ => hubMock.Object);
        services.AddSingleton(_ => grpcFactoryMock.Object);
        services.AddSingleton(_ => healthMonitorMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>((_) => broadcaster);
    }

    protected override async ValueTask SeedDbAsync()
    {
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platformDescriptor = new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 2,
            ContainersStopped: 1);

        var platform = new Platform (
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
        db.Platforms.Add(platform);
        await db.SaveChangesAsync();

        platformId = platform.Id;
    }

    private Guid platformId;

    [Fact]
    public async Task DockerPlatform_Goes_Online_Should_Update_Platform_Info()
    {
        // Arrange
        var platformInfo = new PlatformInfoResponse
        {
            Id = "daemonX",
            ContainerCount = 10,
            ContainersRunning = 5,
            ContainersPaused = 2,
            ContainersStopped = 3,
            NetworkCount = 333,
            VolumeCount = 222,
            ImageCount = 111,
            MemTotal = 4096,
            ServerVersion = "1.2.3",
            AgentVersion = "2.0.0",
            Driver = "overlay2",
            OperatingSystem = "Linux",
            OsType = "linux",
            OsVersion = "5.15",
            Architecture = "x86_64"
        };

        platformClientMock.Setup(x => x.GetPlatformInfoAsync(It.IsAny<Google.Protobuf.WellKnownTypes.Empty>(), null, null, It.IsAny<CancellationToken>()))
                           .Returns(new AsyncUnaryCall<PlatformInfoResponse>(
                                Task.FromResult(platformInfo),
                                Task.FromResult(new Metadata()),
                                () => Status.DefaultSuccess,
                                () => [],
                                () => { }
                            ));

        grpcFactoryMock.Setup(x => x.GetPlatformClient(It.IsAny<string>()))
                        .Returns(platformClientMock.Object);

        grpcFactoryMock.Setup(x => x.GetContainerClient(It.IsAny<string>()))
                        .Returns(containerClientMock.Object);

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true), 
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken); // wait for job to process

        // Assert
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        var platform = await db.Platforms.AsNoTracking().SingleAsync(x => x.Id == platformId, TestContext.Current.CancellationToken);

        hubMock.Verify(x => x.PushPlatformUpdate(It.IsAny<Platform>()), Times.Once);
        await Verify(platform);
    }

    [Fact]
    public async Task DockerPlatform_Goes_Offline_Should_Update_Platform_Info()
    {
        // Arrange
        var platformInfo = new PlatformInfoResponse
        {
            Id = "daemonX",
            ContainerCount = 10,
            ContainersRunning = 5,
            ContainersPaused = 2,
            ContainersStopped = 3,
            NetworkCount = 333,
            VolumeCount = 222,
            ImageCount = 111,
            MemTotal = 4096,
            ServerVersion = "1.2.3",
            AgentVersion = "2.0.0",
            Driver = "overlay2",
            OperatingSystem = "Linux",
            OsType = "linux",
            OsVersion = "5.15",
            Architecture = "x86_64"
        };

        platformClientMock.Setup(x => x.GetPlatformInfoAsync(It.IsAny<Google.Protobuf.WellKnownTypes.Empty>(), null, null, It.IsAny<CancellationToken>()))
                           .Returns(new AsyncUnaryCall<PlatformInfoResponse>(
                                Task.FromResult(platformInfo),
                                Task.FromResult(new Metadata()),
                                () => Status.DefaultSuccess,
                                () => [],
                                () => { }
                            ));

        grpcFactoryMock.Setup(x => x.GetPlatformClient(It.IsAny<string>()))
                        .Returns(platformClientMock.Object);

        grpcFactoryMock.Setup(x => x.GetContainerClient(It.IsAny<string>()))
                        .Returns(containerClientMock.Object);

        // Act
        await broadcaster.BroadcastAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: false),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(1000, TestContext.Current.CancellationToken); // wait for job to process

        // Assert
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        var platform = await db.Platforms.AsNoTracking().SingleAsync(x => x.Id == platformId, TestContext.Current.CancellationToken);

        var options = new JsonSerializerOptions
        {
            DefaultIgnoreCondition = JsonIgnoreCondition.Never // Always include all properties, even if default
        };
        hubMock.Verify(x => x.PushPlatformUpdate(It.IsAny<Platform>()), Times.Once);
        await Verify(platform);
    }
}
