using System.Text;
using System.Net;
using System.Net.Http.Json;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Networks;
using Domain.Entities.Platforms;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;

namespace Tests.Integration.Application.Features.Networks;

public class CreateNetworkTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("0198740b-a501-7ae8-8afc-1e6ce659ee02");
    private readonly Mock<IConnectorFactory<INetworkConnector>> networkFactoryMock = new();
    private readonly Mock<IPlatformContainerCache> platformContainerCacheMock = new();
    private readonly Mock<INetworkConnector> networkConnectorMock = new();
    private readonly Mock<ISwarmReconciliationCoordinator> reconciliationCoordinatorMock = new();
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<ISwarmReconciliationCoordinator>();
        services
            .AddSingleton(networkFactoryMock.Object)
            .AddSingleton(networkConnectorMock.Object)
            .AddSingleton(platformContainerCacheMock.Object)
            .AddSingleton(reconciliationCoordinatorMock.Object);

        networkFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(networkConnectorMock.Object);

        var cacheEntry = new PlatformCacheEntry(PlatformId, "localhost:9000", PlatformConnectorType.Local, []);
        var error = null as Error;
        platformContainerCacheMock.Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out cacheEntry, out error))
            .Returns(true);
        reconciliationCoordinatorMock.Setup(x => x.RefreshAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Platform.FromPersistence(
            PlatformId,
            "network-create-swarm",
            "localhost:9000",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Local,
            platformDescriptor: new DockerSwarmPlatformDescriptor(
                NodeID: "network-create-node",
                NodeAddr: "127.0.0.1",
                LocalNodeState: "Active",
                ControlAvailable: true,
                Nodes: 1,
                Managers: 1,
                DaemonId: "network-create-daemon",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0,
                ClusterId: "network-create-cluster"),
            clusterId: "network-create-cluster");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Create_NetworkIpV4_ReturnsSuccess()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-IPV4-Network"));

        var createJson = $$"""
        {
           "name":"New-IPV4-Network",
           "driver":"bridge",
           "enableIPv4":true,
           "enableIPv6":false,
           "internal":false,
           "attachable":false,
           "ingress":false,
           "options":{

           },
           "labels":{
              "sdfsdf":"sdfsdf",
              "sdfsdfc":"xcvxcv"
           },
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet":"172.21.0.0/16",
                    "ipRange":"172.21.0.0/25",
                    "gateway":"172.21.10.11"
                 },
                 {
                    "subnet":"",
                    "ipRange":"",
                    "gateway":""
                 }
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_NetworkIpV4_FailsOnInvalidGatewayAddress()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-IPV4-Network"));

        var createJson = $$"""
        {
           "name":"New-IPV4-Network",
           "driver":"bridge",
           "enableIPv4":true,
           "enableIPv6":false,
           "internal":false,
           "attachable":false,
           "ingress":false,
           "options":{

           },
           "labels":{},
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet":"172.21.0.0/16",
                    "ipRange":"172.21.0.0/25",
                    "gateway":"175.21.10.11"
                 },
                 {
                    "subnet":"",
                    "ipRange":"",
                    "gateway":""
                 }
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_NetworkIpV6_ReturnsSuccess()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-IPV6-Network"));

        var createJson = $$"""
        {
           "name":"New-IPV6-Network",
           "driver":"bridge",
           "enableIPv4":false,
           "enableIPv6":true,
           "internal":false,
           "attachable":false,
           "ingress":false,
           "options":{

           },
           "labels":{
              "key-label-1":"value-label-1"
           },
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet": null,
                    "ipRange":null,
                    "gateway":null
                 },
                 {
                    "subnet":"fd00::/64",
                    "ipRange":"fd00::1/64",
                    "gateway":"fd00::1"
                 }
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_NetworkIpV6_FailsOnInvalidGatewayAddress()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-IPV6-Network"));

        var createJson = $$"""
        {
           "name":"New-IPV6-Network",
           "driver":"bridge",
           "enableIPv4":false,
           "enableIPv6":true,
           "internal":false,
           "attachable":false,
           "ingress":false,
           "options":{

           },
           "labels":{
              "key-label-1":"value-label-1"
           },
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet": null,
                    "ipRange":null,
                    "gateway":null
                 },
                 {
                    "subnet":"fd00::/64",
                    "ipRange":"fd00::1/64",
                    "gateway":"fg00::1"
                 }
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Network_With_Custom_Driver_ReturnsSuccess()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-Custom-Driver-Network"));

        var createJson = $$"""
        {    
           "name":"New-Custom-Driver-Network",
           "driver":"overlay",
           "scope":"swarm",
           "enableIPv4":true,
           "enableIPv6":false,
           "internal":false,
           "attachable":false,
           "ingress":false,
           "options":{},
           "labels":{},
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet":"172.22.0.0/16"
                 },
                 {}
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Network_With_Internal_And_Attachable_ReturnsSuccess()
    {
        // Arrange
        networkConnectorMock.Setup(x => x.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
           .ReturnsAsync(new CreateDockerNetworkResult("New-Internal-Attachable-Network"));

        var createJson = $$"""
        {
           "name":"Internal-Attachable-Network",
           "driver":"overlay",
           "scope":"swarm",
           "enableIPv4":true,
           "enableIPv6":false,
           "internal":true,
           "attachable":true,
           "ingress":false,
           "options":{},
           "labels":{},
           "ipam":{
              "driver":"default",
              "config":[
                 {
                    "subnet":"172.23.0.0/16"
                 }, 
                 {}
              ]
           },
           "platformId": "0198740b-a501-7ae8-8afc-1e6ce659ee02"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/networks", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await VerifyJson(responseBody);
    }

    [Theory]
    [InlineData("overlay", "local", "Overlay networks must use Swarm scope.")]
    [InlineData("bridge", "swarm", "Swarm-scoped networks must use the overlay driver.")]
    public async Task Create_Network_Rejects_Incompatible_Driver_And_Scope(
        string driver,
        string scope,
        string expectedError)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/networks",
            new
            {
                name = "invalid-network",
                driver,
                scope,
                enableIPv4 = true,
                enableIPv6 = false,
                @internal = false,
                attachable = false,
                ingress = false,
                platformId = "0198740b-a501-7ae8-8afc-1e6ce659ee02"
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        Assert.Contains(expectedError, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        networkConnectorMock.Verify(
            value => value.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }
}
