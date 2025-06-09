using System.Text;
using Citadel.Agent.Common.V1;
using Grpc.Core;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.Entities.Platforms;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using static Citadel.Agent.Platforms.V1.PlatformService;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid platformId;
    private readonly Mock<IGrpcClientFactory> grpcFactoryMock = new();
    private readonly Mock<PlatformServiceClient> platformClientMock = new();
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => grpcFactoryMock.Object);
        services.AddSingleton(_ => healthMonitorMock.Object);
    }

    protected override async ValueTask SeedDbAsync()
    {
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platform = new Platform(
            name: "P-01",
            address: "https://original.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            type: PlatformType.Docker,
            status: PlatformStatus.Online,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "123456",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1)
        );
        db.Platforms.Add(platform);

        await db.SaveChangesAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task Patch_Platform_Should_Update_Entity()
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
        platformClientMock.Setup(x => x.ListPlatformInfoAsync(It.IsAny<Google.Protobuf.WellKnownTypes.Empty>(), null, null, It.IsAny<CancellationToken>()))
                           .Returns(new AsyncUnaryCall<PlatformInfoResponse>(
                                Task.FromResult(platformInfo),
                                Task.FromResult(new Metadata()),
                                () => Status.DefaultSuccess,
                                () => [],
                                () => { }
                            ));

        healthMonitorMock.Setup(x => x.UntrackPlatform("https://original.address", It.IsAny<CancellationToken>()))
                         .ReturnsAsync(true);

        grpcFactoryMock.Setup(x => x.GetPlatformClient(It.IsAny<string>()))
                        .Returns(platformClientMock.Object);

        var patchJson = """
        {
          "name": "P-02",
          "type": "Docker",
          "address": "https://localhost:9000"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        
        // Act

        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Assert
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        var platform = await db.Platforms.AsNoTracking().SingleAsync(x => x.Id == platformId, TestContext.Current.CancellationToken);

        healthMonitorMock.Verify(x => x.TrackPlatform("https://localhost:9000", platformId), Times.Once);
        await Verify(platform);
    }

    [Fact]
    public async Task Patch_Platform_Should_Return_Forbidden_If_User_Lacks_Permission()
    {
        // Arrange
        var patchJson = """
        {
          "name": "P-02",
          "type": "Docker",
          "address": "https://localhost:9000"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var token = CreateJwtTokenAsync([]);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Platform_Should_Return_NotFound_If_Id_Does_Not_Exist()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "name": "P-03"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{nonExistentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Platform_Should_Return_Conflict_If_Name_Already_Exists()
    {
        // Arrange
        // Add another platform with a conflicting name
        using (var scope = Services.CreateScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            db.Platforms.Add(new Platform(
                name: "P-02",
                address: "https://another.address",
                networkCount: 1,
                volumeCount: 2,
                imageCount: 3,
                cpuCount: 4,
                memTotal: 500,
                serverVersion: "1.0.0",
                agentVersion: "1.0.0",
                type: PlatformType.Docker,
                status: PlatformStatus.Online,
                platformDescriptor: new DockerPlatformDescriptor(
                    DaemonId: "654321",
                    ContainerCount: 5,
                    ContainersRunning: 2,
                    ContainersPaused: 2,
                    ContainersStopped: 1)
            ));
            await db.SaveChangesAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = """
        {
          "name": "P-02"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Platform_Should_Return_BadRequest_If_Payload_Is_Invalid()
    {
        // Arrange
        var patchJson = """
        {
          "name": "sdc@éà",
          "address": "invalid-address::9000"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Platform_Should_Return_Unauthorized_If_No_Token()
    {
        // Arrange
        var patchJson = """
        {
          "name": "P-04"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        Client.DefaultRequestHeaders.Authorization = null;

        // Act
        var response = await Client.PatchAsync($"/api/v1/platforms/{platformId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Unauthorized, response.StatusCode);
    }
}
