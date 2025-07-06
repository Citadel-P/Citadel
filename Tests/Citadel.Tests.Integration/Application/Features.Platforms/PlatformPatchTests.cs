using System.Text;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid platformId;

    private readonly Mock<IConnectorFactory<IPlatformConnector>> platformFactoryMock = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();

    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => healthMonitorMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => platformConnector.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
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
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "123456",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1)
        );

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task Patch_Platform_Should_Update_Entity()
    {
        // Arrange
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                     .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(GetDummyPlatformResult()));

        healthMonitorMock.Setup(x => x.UntrackPlatform("https://original.address", It.IsAny<CancellationToken>()))
                         .ReturnsAsync(true);

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
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByIdAsync(platformId, TestContext.Current.CancellationToken);

        healthMonitorMock.Verify(x => x.TrackPlatform("https://localhost:9000", platformId, PlatformConnectorType.Agent), Times.Once);
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
        await using var scope = Services.CreateAsyncScope();
        var platform = new Platform(
            name: "P-02",
            address: "https://another.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "654321",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1)
        );
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var r = await uow.Platforms.GetPlatformsWithLatestStatAsync(TestContext.Current.CancellationToken);
        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync();
        var r2 = await uow.Platforms.GetPlatformsWithLatestStatAsync(TestContext.Current.CancellationToken);

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
