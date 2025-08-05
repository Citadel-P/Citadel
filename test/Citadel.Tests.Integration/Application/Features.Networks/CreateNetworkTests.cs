using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Networks;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Networks;

public class CreateNetworkTests : IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<INetworkConnector>> networkFactoryMock = new();
    private readonly Mock<IPlatformContainerCache> platformContainerCacheMock = new();
    private readonly Mock<INetworkConnector> networkConnectorMock = new();
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services
            .AddSingleton(networkFactoryMock.Object)
            .AddSingleton(networkConnectorMock.Object)
            .AddSingleton(platformContainerCacheMock.Object);

        networkFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(networkConnectorMock.Object);

        var cacheEntry = new PlatformCacheEntry("localhost:9000", PlatformConnectorType.Local, []);
        platformContainerCacheMock.Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out cacheEntry))
            .Returns(true);
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
}
