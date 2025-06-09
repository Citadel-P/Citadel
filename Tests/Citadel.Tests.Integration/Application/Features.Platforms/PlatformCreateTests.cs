using System.Text;
using Citadel.Agent.Common.V1;
using Citadel.Agent.Containers.V1;
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
using static Citadel.Agent.Containers.V1.ContainerService;
using static Citadel.Agent.Platforms.V1.PlatformService;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformCreateTests : IntegrationTestBase<WebApi.Program>
{
    private readonly Mock<IGrpcClientFactory> grpcFactoryMock = new();
    private readonly Mock<PlatformServiceClient> platformClientMock = new();
    private readonly Mock<ContainerServiceClient> containersClientMock = new();
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => grpcFactoryMock.Object);
        services.AddSingleton(_ => healthMonitorMock.Object);
    }

    [Fact]
    public async Task CreatePlatform_Should_Succeed()
    {
        // Arrange
        var platformInfo = new PlatformInfoMessage
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

        var containersReply = new ListContainersResponse
        {
            Containers =
            {
                ["containerId1"] = new ContainerMessage { Id = "c1", Name = "Container 1" },
                ["containerId2"] = new ContainerMessage { Id = "c2", Name = "Container 2" },
                ["containerId3"] = new ContainerMessage { Id = "c3", Name = "Container 3" }
            }
        };

        platformClientMock.Setup(x => x.ListPlatformInfoAsync(It.IsAny<Google.Protobuf.WellKnownTypes.Empty>(), null, null, It.IsAny<CancellationToken>()))
            .Returns(new AsyncUnaryCall<PlatformInfoMessage>(
                Task.FromResult(platformInfo),
                Task.FromResult(new Metadata()),
                () => Status.DefaultSuccess,
                () => [],
                () => { }
            ));

        containersClientMock.Setup(x => x.ListAsync(It.IsAny<ListContainersMessage>(), null, null, It.IsAny<CancellationToken>()))
            .Returns(new AsyncUnaryCall<ListContainersResponse>(
                Task.FromResult(containersReply),
                Task.FromResult(new Metadata()),
                () => Status.DefaultSuccess,
                () => [],
                () => { }
            ));

        grpcFactoryMock.Setup(x => x.GetPlatformClient(It.IsAny<string>()))
            .Returns(platformClientMock.Object);

        grpcFactoryMock.Setup(x => x.GetContainerClient(It.IsAny<string>()))
            .Returns(containersClientMock.Object);

        healthMonitorMock.Setup(x => x.TrackPlatform(It.IsAny<string>(), It.IsAny<Guid>())).Returns(true);

        var createJson = """
        {
          "name": "P-NEW",
          "address": "https://localhost:9000",
          "type": "Docker"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        // Check DB
        using var scope = Services.CreateScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        var platform = await db.Platforms.AsNoTracking().SingleOrDefaultAsync(x => x.Name == "P-NEW", TestContext.Current.CancellationToken);
        var containers = await db.Containers.AsNoTracking().ToListAsync(TestContext.Current.CancellationToken);
        
        Assert.NotNull(platform);
        Assert.Equal(3, containers.Count);
        healthMonitorMock.Verify(x => x.TrackPlatform("https://localhost:9000", platform.Id), Times.Once);
        await Verify(platform);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_Conflict_If_Name_Or_Address_Exists()
    {
        // Arrange: Seed a platform
        using (var scope = Services.CreateScope())
        {
            var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            db.Platforms.Add(new Platform(
                name: "P-EXIST",
                address: "https://localhost:9001",
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

        var createJson = """
        {
          "name": "P-EXIST",
          "address": "https://localhost:9002",
          "type": "Docker"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_BadRequest_If_Payload_Is_Invalid()
    {
        // Arrange
        var createJson = """
        {
          "name": "",
          "address": "invalid-address",
          "type": "Docker"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_Forbidden_If_User_Lacks_Permission()
    {
        // Arrange
        var createJson = """
        {
          "name": "P-FORBIDDEN",
          "address": "https://localhost:9003",
          "type": "Docker"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        var token = CreateJwtTokenAsync([]); // No permissions
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_Unauthorized_If_No_Token()
    {
        // Arrange
        var createJson = """
        {
          "name": "P-UNAUTH",
          "address": "https://localhost:9004",
          "type": "Docker"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");
        Client.DefaultRequestHeaders.Authorization = null;

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.Unauthorized, response.StatusCode);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_BadRequest_If_Unsupported_Type()
    {
        // Arrange
        var createJson = """
        {
          "name": "P-UNSUPPORTED",
          "address": "https://localhost:9005",
          "type": "Kubernetes"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

}
