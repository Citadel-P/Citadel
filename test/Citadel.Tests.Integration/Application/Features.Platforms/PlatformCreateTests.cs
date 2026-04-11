using System.Text;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IConnectorFactory<IPlatformConnector>> platformFactoryMock = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerFactoryMock = new();
    private readonly Mock<IConnectorFactory<IImageConnector>> imageFactoryMock = new();
    private readonly Mock<IImageConnector> imageConnector = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();
    private readonly Mock<IContainerConnector> containerConnector = new();
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => healthMonitorMock.Object);
        services.AddSingleton(_ => imageFactoryMock.Object);
        services.AddSingleton(_ => platformFactoryMock.Object);
        services.AddSingleton(_ => containerFactoryMock.Object);
        services.AddSingleton(_ => imageConnector.Object);
        services.AddSingleton(_ => platformConnector.Object);
        services.AddSingleton(_ => containerConnector.Object);
    }

    [Fact]
    public async Task CreatePlatform_WithAgentConnector_ReturnsSuccessAndPersistsContainers()
    {
        // Arrange
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                      .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(Fakes.GetDummyPlatformResult()));

        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(containerConnector.Object);

        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(Fakes.GetDummyContainers().ToDictionary(c => c.Id)));

        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<ImageResult>>(Fakes.GetDummyImages()));

        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(imageConnector.Object);

        healthMonitorMock.Setup(x => x.TrackPlatform(It.IsAny<string>(), It.IsAny<Guid>(), It.IsAny<PlatformConnectorType>())).Returns(true);

        var createJson = """
        {
          "name": "P-NEW",
          "address": "https://localhost:9000",
          "type": "Docker",
          "connectorType": "agent"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        // Assert
        response.EnsureSuccessStatusCode();

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByNameAsync("P-NEW", TestContext.Current.CancellationToken) ?? throw new Exception("platform can not be null");
        var containers = await uow.Containers.GetContainersInfoAsync(platform.Id, TestContext.Current.CancellationToken);
        var images = await uow.Images.GetByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(platform);
        Assert.Equal(3, images.Count());
        Assert.Equal(3, containers?.Count());
        // Containers has foreign key on Images table
        Assert.All(containers, c => Assert.Contains(c.ImageId.Value, images.Select(i => i.Id)));
        healthMonitorMock.Verify(x => x.TrackPlatform("https://localhost:9000", platform.Id, PlatformConnectorType.Agent), Times.Once);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task CreatePlatform_WithMLocalConnector_ReturnsSuccessAndPersistsContainers()
    {
        // Arrange
        platformFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
                      .Returns(platformConnector.Object);

        platformConnector.Setup(x => x.GetPlatformAsync(It.IsAny<GetPlatformCommand>(), It.IsAny<CancellationToken>()))
                         .ReturnsAsync(Result.Success(Fakes.GetDummyPlatformResult()));

        containerFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(containerConnector.Object);

        containerConnector.Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(Fakes.GetDummyContainers().ToDictionary(c => c.Id)));

        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Fakes.GetDummyImages()));

        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(imageConnector.Object);

        healthMonitorMock.Setup(x => x.TrackPlatform(It.IsAny<string>(), It.IsAny<Guid>(), It.IsAny<PlatformConnectorType>())).Returns(true);

        var createJson = """
        {
          "name": "P-NEW",
          "type": "Docker",
          "connectorType": "local"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/platforms", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        // Assert
        response.EnsureSuccessStatusCode();

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await uow.Platforms.GetByNameAsync("P-NEW", TestContext.Current.CancellationToken) ?? throw new Exception("platform can not be null");
        var containers = await uow.Containers.GetContainersInfoAsync(platform.Id, TestContext.Current.CancellationToken) ?? throw new Exception("containers can not be null");
        var images = await uow.Images.GetByPlatformIdAsync(platform.Id, TestContext.Current.CancellationToken);

        Assert.NotNull(platform);
        Assert.Equal(3, images.Count());
        Assert.Equal(3, containers?.Count());
        // Containers has foreign key on Images table
        Assert.All(containers, c => Assert.Contains(c.ImageId.Value, images.Select(i => i.Id)));
        healthMonitorMock.Verify(x => x.TrackPlatform(Constants.LocalDockerHostUrl, platform.Id, PlatformConnectorType.Local), Times.Once);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task CreatePlatform_Should_Return_Conflict_If_Name_Or_Address_Exists()
    {
        // Arrange: Seed a platform
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = new Platform(
            name: "P-EXIST",
            address: "https://localhost:9001",
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
                ContainersStopped: 1));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

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
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
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
          "type": "Docker",
          "connectorType": "agent"
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
        var subject = await CreateAuthorizationSubjectAsync();
        var token = CreateJwtToken(subject.UserId, subject.ActorId);
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
