using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Volumes;

public sealed class VolumeEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("019c24c9-4cf7-7ef7-ae4f-ccdc885d65d0");
    private readonly Mock<IConnectorFactory<IVolumeConnector>> connectorFactory = new();
    private readonly Mock<IVolumeConnector> connector = new();
    private readonly Mock<IVolumeContentService> contentService = new();
    private readonly Mock<IPlatformContainerCache> platformCache = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        var cacheEntry = new PlatformCacheEntry(
            PlatformId,
            "docker.test:2375",
            PlatformConnectorType.Agent,
            []);
        Error? cacheError = null;
        platformCache
            .Setup(cache => cache.TryGetCacheEntry(PlatformId, out cacheEntry, out cacheError))
            .Returns(true);
        connectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(connector.Object);

        services.ReplaceService<IPlatformContainerCache>(platformCache.Object);
        services.ReplaceService<IConnectorFactory<IVolumeConnector>>(connectorFactory.Object);
        services.ReplaceService<IVolumeContentService>(contentService.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Platform.FromPersistence(
            PlatformId,
            "volume-endpoint-platform",
            "docker.test:2375",
            networkCount: 0,
            volumeCount: 1,
            imageCount: 0,
            cpuCount: 2,
            memTotal: 2048,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor("volume-endpoint-daemon", 0, 1, 0, 0));
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task VolumeEndpoints_ShouldMapPlatformAndForwardDaemonOperations()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        connector
            .Setup(value => value.ListVolumesAsync(
                It.Is<ListdDockerVolumesCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Dangling == true
                    && command.Driver == "local"
                    && command.Name == "data"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IEnumerable<DockerVolumeResult>>([Volume("data")]));
        connector
            .Setup(value => value.InspectVolumeAsync(
                It.Is<InspectDockerVolumeCommand>(command =>
                    command.PlatformAddress == "docker.test:2375" && command.Name == "data"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Volume("data")));
        connector
            .Setup(value => value.CreateVolumeAsync(
                It.Is<CreateDockerVolumeCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Name == "created-data"
                    && command.Driver == "local"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Volume("created-data")));
        connector
            .Setup(value => value.DeleteVolumeAsync(
                It.Is<DeleteDockerVolumeCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.Names.SequenceEqual(new[] { "created-data" })
                    && command.Force == true),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        using var listResponse = await Client.GetAsync(
            $"/api/v1/volumes/{PlatformId:D}?dangling=true&driver=local&name=data",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var listDocument = JsonDocument.Parse(await listResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Equal("data", listDocument.RootElement.GetProperty("volumes")[0].GetProperty("name").GetString());

        using var inspectResponse = await Client.GetAsync(
            $"/api/v1/volumes/{PlatformId:D}/data",
            cancellationToken);
        inspectResponse.EnsureSuccessStatusCode();

        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/volumes",
            new
            {
                platformId = PlatformId,
                name = "created-data",
                driver = "local",
                labels = new Dictionary<string, string> { ["purpose"] = "integration" },
                options = new Dictionary<string, string>()
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();

        using var deleteRequest = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/volumes")
        {
            Content = JsonContent.Create(new
            {
                platformId = PlatformId,
                names = new[] { "created-data" },
                force = true
            })
        };
        using var deleteResponse = await Client.SendAsync(deleteRequest, cancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        connector.VerifyAll();
    }

    [Fact]
    public async Task VolumeContentEndpoints_ShouldStreamContentAndPersistDownloadActivity()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        connector
            .Setup(value => value.InspectVolumeAsync(
                It.Is<InspectDockerVolumeCommand>(command =>
                    command.PlatformAddress == "docker.test:2375" && command.Name == "data"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(Volume("data")));
        contentService
            .Setup(value => value.ListDirectoryAsync(
                It.Is<ListVolumeDirectoryCommand>(command =>
                    command.PlatformId == PlatformId
                    && command.VolumeName == "data"
                    && command.Path.ApiPath == "/logs"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new VolumeDirectoryListing(
                PlatformId,
                "data",
                "logs",
                [new VolumeFileEntry("app.log", "logs/app.log", VolumeFileEntryType.File, 5, null, null)],
                IsTruncated: false)));
        contentService
            .Setup(value => value.OpenDownloadAsync(
                It.Is<DownloadVolumePathCommand>(command =>
                    command.PlatformId == PlatformId
                    && command.VolumeName == "data"
                    && command.Path.ApiPath == "/logs/app.log"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new VolumeDownloadStream
            {
                PlatformId = PlatformId,
                VolumeName = "data",
                Path = "logs/app.log",
                EntryType = VolumeFileEntryType.File,
                FileName = "app.log",
                ContentType = "text/plain",
                ContentLength = 5,
                Chunks = DownloadChunks(),
                CleanupAsync = static () => ValueTask.CompletedTask
            }));

        using var listResponse = await Client.GetAsync(
            $"/api/v1/platforms/{PlatformId:D}/volumes/data/files?path=/logs",
            cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var listing = JsonDocument.Parse(await listResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Equal("app.log", listing.RootElement.GetProperty("entries")[0].GetProperty("name").GetString());

        using var downloadResponse = await Client.GetAsync(
            $"/api/v1/platforms/{PlatformId:D}/volumes/data/files/download?path=/logs/app.log",
            cancellationToken);
        downloadResponse.EnsureSuccessStatusCode();
        Assert.Equal("hello", await downloadResponse.Content.ReadAsStringAsync(cancellationToken));
        Assert.Equal("attachment", downloadResponse.Content.Headers.ContentDisposition?.DispositionType);
        Assert.Equal("app.log", downloadResponse.Content.Headers.ContentDisposition?.FileName);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            PlatformId,
            ActivityResourceType.Volume,
            ActivityEventType.VolumeContentDownloaded,
            page: 1,
            pageSize: 10,
            cancellationToken);
        Assert.Single(activities.Items);

        connector.VerifyAll();
        contentService.VerifyAll();
    }

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> DownloadChunks()
    {
        yield return Encoding.UTF8.GetBytes("hello");
        await Task.CompletedTask;
    }

    private static DockerVolumeResult Volume(string name)
        => new(
            Id: name,
            Name: name,
            InUse: false,
            Scope: "local",
            Driver: "local",
            Mountpoint: $"/var/lib/docker/volumes/{name}/_data",
            CreatedAt: "2026-08-04T00:00:00Z",
            ClusterVolume: null,
            UsageData: null,
            Containers: [],
            Status: new Dictionary<string, string>(),
            Labels: new Dictionary<string, string>(),
            Options: new Dictionary<string, string>());
}
