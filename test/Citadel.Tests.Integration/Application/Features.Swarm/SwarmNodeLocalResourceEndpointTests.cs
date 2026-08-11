using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class SwarmNodeLocalResourceEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("019c2b96-8143-7dbd-a9b3-83622355b56f");
    private const string DockerNodeId = "worker-node";
    private const string DockerImageId = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private const string DockerNetworkId = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    private readonly Mock<ISwarmNodeRuntimeConnector> runtimeConnector = new();
    private readonly Mock<INetworkConnector> networkConnector = new();
    private readonly Mock<IVolumeContentService> volumeContentService = new();
    private readonly Mock<IPlatformContainerCache> platformContainerCache = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        var cacheEntry = new PlatformCacheEntry(
            PlatformId,
            "docker.test:2375",
            PlatformConnectorType.Agent,
            []);
        Error? cacheError = null;
        platformContainerCache
            .Setup(value => value.TryGetCacheEntry(PlatformId, out cacheEntry, out cacheError))
            .Returns(true);

        var networkFactory = new Mock<IConnectorFactory<INetworkConnector>>();
        networkFactory.Setup(value => value.GetConnector(PlatformConnectorType.Agent)).Returns(networkConnector.Object);
        services.ReplaceService(platformContainerCache.Object);
        services.ReplaceService(networkFactory.Object);
        services.ReplaceService(runtimeConnector.Object);
        services.ReplaceService(volumeContentService.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var observedAt = DateTimeOffset.UtcNow;
        var platform = Platform.FromPersistence(
            PlatformId,
            "swarm-node-resources",
            "docker.test:2375",
            0, 0, 0, 2, 2048,
            PlatformStatus.Online,
            PlatformConnectorType.Agent,
            new DockerSwarmPlatformDescriptor(
                "manager-node", "10.0.0.1", "Active", true, 2, 1,
                "daemon-id", 0, 0, 0, 0, "cluster-id"),
            clusterId: "cluster-id");
        var node = new SwarmNodeProjection(
            PlatformId, DockerNodeId, 1, "worker-1", "Worker", false, "", "Ready", null,
            "Active", "28.0", "linux", "x86_64", "10.0.0.2", new Dictionary<string, string>(),
            0, 0, observedAt, observedAt, observedAt, false);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        await uow.Swarm.ReplaceAsync(
            PlatformId,
            new SwarmProjectionSnapshot([node], [], [], [], [], []),
            TestContext.Current.CancellationToken);
        await uow.Swarm.ReplaceNodeLocalResourcesAsync(
            PlatformId,
            DockerNodeId,
            [SwarmNodeImageProjection.FromObservation(PlatformId, DockerNodeId, Image(), observedAt)],
            [SwarmNodeVolumeProjection.FromObservation(PlatformId, DockerNodeId, Volume(), observedAt)],
            [SwarmNodeNetworkProjection.FromObservation(PlatformId, DockerNodeId, Network(), observedAt)],
            observedAt.AddMilliseconds(-1),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task NodeLocalReadEndpoints_ShouldRouteToOwningNodeWithoutPlatformCache()
    {
        runtimeConnector
            .Setup(value => value.InspectImageAsync(
                It.IsAny<Platform>(), DockerNodeId, DockerImageId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectedImage()));
        runtimeConnector
            .Setup(value => value.InspectVolumeAsync(
                It.IsAny<Platform>(), DockerNodeId, "data", It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Volume()));
        runtimeConnector
            .Setup(value => value.InspectNetworkAsync(
                It.IsAny<Platform>(), DockerNodeId, DockerNetworkId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(NetworkDetails()));
        volumeContentService
            .Setup(value => value.ListDirectoryAsync(
                It.Is<ListVolumeDirectoryCommand>(command =>
                    command.PlatformId == PlatformId
                    && command.VolumeName == "data"
                    && command.DockerNodeId == DockerNodeId
                    && command.Platform != null),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new VolumeDirectoryListing(
                PlatformId, "data", string.Empty, [], IsTruncated: false)));
        volumeContentService
            .Setup(value => value.OpenDownloadAsync(
                It.Is<DownloadVolumePathCommand>(command =>
                    command.PlatformId == PlatformId
                    && command.VolumeName == "data"
                    && command.DockerNodeId == DockerNodeId
                    && command.Platform != null
                    && command.Path.ApiPath == "/app.log"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new VolumeDownloadStream
            {
                PlatformId = PlatformId,
                VolumeName = "data",
                Path = "app.log",
                EntryType = VolumeFileEntryType.File,
                FileName = "app.log",
                ContentType = "text/plain",
                ContentLength = 5,
                Chunks = DownloadChunks(),
                CleanupAsync = static () => ValueTask.CompletedTask
            }));

        platformContainerCache.Reset();
        await AssertListContainsNodeAsync($"/api/v1/images/{PlatformId:D}", "images");
        await AssertListContainsNodeAsync($"/api/v1/volumes/{PlatformId:D}", "volumes");
        await AssertListContainsNodeAsync($"/api/v1/networks/{PlatformId:D}", "networks");

        await AssertInspectContainsNodeAsync($"/api/v1/images/{PlatformId:D}/{DockerImageId}?dockerNodeId={DockerNodeId}");
        await AssertInspectContainsNodeAsync($"/api/v1/volumes/{PlatformId:D}/data?dockerNodeId={DockerNodeId}");
        await AssertInspectContainsNodeAsync($"/api/v1/networks/{PlatformId:D}/{DockerNetworkId}?dockerNodeId={DockerNodeId}");

        using var browseResponse = await Client.GetAsync(
            $"/api/v1/platforms/{PlatformId:D}/volumes/data/files?dockerNodeId={DockerNodeId}",
            TestContext.Current.CancellationToken);
        browseResponse.EnsureSuccessStatusCode();

        using var downloadResponse = await Client.GetAsync(
            $"/api/v1/platforms/{PlatformId:D}/volumes/data/files/download?path=/app.log&dockerNodeId={DockerNodeId}",
            TestContext.Current.CancellationToken);
        downloadResponse.EnsureSuccessStatusCode();
        Assert.Equal("hello", await downloadResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));

        platformContainerCache.VerifyNoOtherCalls();
        volumeContentService.VerifyAll();

        using var overviewResponse = await Client.GetAsync(
            $"/api/v1/platforms/{PlatformId:D}/swarm",
            TestContext.Current.CancellationToken);
        overviewResponse.EnsureSuccessStatusCode();
        using var overview = JsonDocument.Parse(await overviewResponse.Content.ReadAsStreamAsync(
            TestContext.Current.CancellationToken));
        Assert.Equal(1, overview.RootElement.GetProperty("imageCount").GetInt32());
        Assert.Equal(1, overview.RootElement.GetProperty("volumeCount").GetInt32());
        Assert.Equal(1, overview.RootElement.GetProperty("localNetworkCount").GetInt32());
        Assert.Equal(1, overview.RootElement.GetProperty("networkCount").GetInt32());
    }

    [Fact]
    public async Task NodeLocalResources_ShouldRejectAmbiguousManagerDeleteRequests()
    {
        using var imageResponse = await DeleteAsync("/api/v1/images", new
        {
            platformId = PlatformId,
            ids = new[] { DockerImageId },
            force = true
        });
        using var volumeResponse = await DeleteAsync("/api/v1/volumes", new
        {
            platformId = PlatformId,
            names = new[] { "data" },
            force = true
        });
        using var networkResponse = await DeleteAsync("/api/v1/networks", new
        {
            platformId = PlatformId,
            ids = new[] { DockerNetworkId }
        });

        Assert.Equal(HttpStatusCode.Conflict, imageResponse.StatusCode);
        Assert.Equal(HttpStatusCode.Conflict, volumeResponse.StatusCode);
        Assert.Equal(HttpStatusCode.Conflict, networkResponse.StatusCode);

        networkConnector.Verify(
            value => value.DeleteNetworkAsync(It.IsAny<DeleteDockerNetworkCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private async Task AssertListContainsNodeAsync(string url, string property)
    {
        using var response = await Client.GetAsync(url, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();
        using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(
            TestContext.Current.CancellationToken));
        var resource = Assert.Single(document.RootElement.GetProperty(property).EnumerateArray());
        Assert.Equal(DockerNodeId, resource.GetProperty("dockerNodeId").GetString());
        Assert.Equal("worker-1", resource.GetProperty("nodeHostname").GetString());
        Assert.False(resource.GetProperty("isStale").GetBoolean());
    }

    private async Task AssertInspectContainsNodeAsync(string url)
    {
        using var response = await Client.GetAsync(url, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();
        using var document = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(
            TestContext.Current.CancellationToken));
        Assert.Equal(DockerNodeId, document.RootElement.GetProperty("dockerNodeId").GetString());
    }

    private async Task<HttpResponseMessage> DeleteAsync(string url, object body) => await Client.SendAsync(
        new HttpRequestMessage(HttpMethod.Delete, url) { Content = JsonContent.Create(body) },
        TestContext.Current.CancellationToken);

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> DownloadChunks()
    {
        yield return "hello"u8.ToArray();
        await Task.CompletedTask;
    }

    private static ImageResult Image() => new(
        DockerImageId, 100, 0, string.Empty, 0, 1_700_000_000, 100,
        ["redis:latest"], ["redis@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"],
        new Dictionary<string, string>());

    private static InspectImageResult InspectedImage() => new(
        DockerImageId, 100, "linux", "2026-08-11T00:00:00Z", "amd64",
        [], [], ["redis:latest"], [], [], [], new Dictionary<string, string>(), []);

    private static DockerVolumeResult Volume() => new(
        "data", "data", false, "local", "local", "/var/lib/docker/volumes/data",
        "2026-08-11T00:00:00Z", null, null, [], new Dictionary<string, string>(),
        new Dictionary<string, string>(), new Dictionary<string, string>());

    private static DockerNetworkResult Network() => new(
        "bridge", DockerNetworkId, "2026-08-11T00:00:00Z", "bridge", "local",
        true, false, false, false, false, false, false, null, null,
        new Dictionary<string, string>(), new Dictionary<string, string>());

    private static DockerNetworkDetails NetworkDetails() => new(
        "bridge", DockerNetworkId, "2026-08-11T00:00:00Z", "bridge", "local",
        true, false, false, false, false, false, null, null,
        new Dictionary<string, string>(), new Dictionary<string, string>(),
        new Dictionary<string, NetworkConnectedContainer>(), []);
}
