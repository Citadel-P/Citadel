using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Networks;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Networks;

public sealed class NetworkEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("019c2513-ccda-71d4-a719-3141292bcf65");
    private const string NetworkId = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private readonly Mock<IConnectorFactory<INetworkConnector>> connectorFactory = new();
    private readonly Mock<INetworkConnector> connector = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        var cache = new Mock<IPlatformContainerCache>();
        var cacheEntry = new PlatformCacheEntry(
            PlatformId,
            "docker.test:2375",
            PlatformConnectorType.Agent,
            []);
        Error? cacheError = null;
        cache
            .Setup(value => value.TryGetCacheEntry(PlatformId, out cacheEntry, out cacheError))
            .Returns(true);
        connectorFactory
            .Setup(value => value.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);

        services.ReplaceService<IPlatformContainerCache>(cache.Object);
        services.ReplaceService<IConnectorFactory<INetworkConnector>>(connectorFactory.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Platform.FromPersistence(
            PlatformId,
            "network-endpoint-platform",
            "docker.test:2375",
            networkCount: 1,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor("network-endpoint-daemon", 1, 0, 0, 0));
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task NetworkEndpoints_ShouldMapPlatformAndForwardDaemonOperations()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var listedNetworkResult = Network();
        connector
            .Setup(value => value.ListNetworksAsync(
                It.Is<ListNetworksCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Dangling == false
                    && command.Driver == "bridge"
                    && command.Name == "app"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IEnumerable<DockerNetworkResult>>([listedNetworkResult]));
        connector
            .Setup(value => value.InspectNetworkAsync(
                It.Is<InspectNetworkCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.NetworkId == NetworkId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(NetworkDetails()));
        connector
            .Setup(value => value.DeleteNetworkAsync(
                It.Is<DeleteDockerNetworkCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Ids.SequenceEqual(new[] { NetworkId })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        using var listResponse = await Client.GetAsync(
            $"/api/v1/networks/{PlatformId:D}?dangling=false&driver=bridge&name=app",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var list = JsonDocument.Parse(await listResponse.Content.ReadAsStreamAsync(cancellationToken));
        var listedNetwork = list.RootElement.GetProperty("networks")[0];
        Assert.Equal("app", listedNetwork.GetProperty("name").GetString());
        Assert.Equal(PlatformId, listedNetworkResult.PlatformId);

        using var inspectResponse = await Client.GetAsync(
            $"/api/v1/networks/{PlatformId:D}/{NetworkId}",
            cancellationToken);
        inspectResponse.EnsureSuccessStatusCode();
        using var inspect = JsonDocument.Parse(await inspectResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Equal("app", inspect.RootElement.GetProperty("name").GetString());

        using var deleteRequest = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/networks")
        {
            Content = JsonContent.Create(new
            {
                platformId = PlatformId,
                ids = new[] { NetworkId }
            })
        };
        using var deleteResponse = await Client.SendAsync(deleteRequest, cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        connector.VerifyAll();
    }

    private static DockerNetworkResult Network()
        => new(
            "app",
            NetworkId,
            "2026-08-04T00:00:00Z",
            "bridge",
            "local",
            EnableIPv4: true,
            EnableIPv6: false,
            Internal: false,
            Attachable: false,
            Ingress: false,
            ConfigOnly: false,
            InUse: false,
            ConfigFrom: null,
            Ipam: null,
            Options: new Dictionary<string, string>(),
            Labels: new Dictionary<string, string>());

    private static DockerNetworkDetails NetworkDetails()
        => new(
            "app",
            NetworkId,
            "2026-08-04T00:00:00Z",
            "bridge",
            "local",
            EnableIPv4: true,
            EnableIPv6: false,
            Internal: false,
            Attachable: false,
            Ingress: false,
            ConfigOnly: false,
            ConfigFrom: null,
            Ipam: null,
            Options: new Dictionary<string, string>(),
            Labels: new Dictionary<string, string>(),
            Containers: new Dictionary<string, NetworkConnectedContainer>(),
            Peers: []);
}
