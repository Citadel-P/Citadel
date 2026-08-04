using System.Net.Http.Json;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Threading.Channels;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Registries;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Images;

public sealed class ImageEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid PlatformId = Guid.Parse("019c24f0-1f04-7e23-b9e7-46597aa21f39");
    private const string SeededDockerImageId = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private const string PulledDockerImageId = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    private readonly Mock<IConnectorFactory<IImageConnector>> connectorFactory = new();
    private readonly Mock<IImageConnector> connector = new();
    private readonly Mock<IDockerHubRegistryRepository> dockerHub = new();
    private readonly Mock<IGitHubCrRepository> github = new();
    private Guid imageId;
    private Guid dockerRegistryId;

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
        services.ReplaceService<IConnectorFactory<IImageConnector>>(connectorFactory.Object);
        services.ReplaceService<IDockerHubRegistryRepository>(dockerHub.Object);
        services.ReplaceService<IGitHubCrRepository>(github.Object);
        services.RemoveService<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue, InlineDbWorkQueue>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Platform.FromPersistence(
            PlatformId,
            "image-endpoint-platform",
            "docker.test:2375",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 1,
            cpuCount: 2,
            memTotal: 2048,
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor("image-endpoint-daemon", 0, 0, 1, 0));
        var dockerRegistry = new Registry(
            "docker-endpoint",
            "registry-1.docker.io",
            RegistryStatus.Active,
            Constants.DefaultAdminId,
            new DockerHubRegistry("tester", "token"));
        var githubRegistry = new Registry(
            "github-endpoint",
            "ghcr.io",
            RegistryStatus.Active,
            Constants.DefaultAdminId,
            new GitHubRegistry("citadel", true, "token"));
        var image = new Image(
            "nginx",
            ["nginx:stable"],
            SeededDockerImageId,
            1024,
            0,
            PlatformId,
            DateTime.UtcNow,
            registryId: dockerRegistry.Id,
            registry: dockerRegistry);
        imageId = image.Id;
        dockerRegistryId = dockerRegistry.Id;

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Registries.AddAsync(dockerRegistry, TestContext.Current.CancellationToken);
        await uow.Registries.AddAsync(githubRegistry, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task ImageReadAndRegistryEndpoints_ShouldReturnConnectorAndPersistedData()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        dockerHub
            .Setup(value => value.GetRepositoriesAsync(
                It.IsAny<DockerHubRegistry>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((
                [new DockerHubRepositoryInfo("nginx", "tester", DateTime.UtcNow, false, false, false, 12)],
                null));
        dockerHub
            .Setup(value => value.GetRepositoryTagsAsync(
                It.IsAny<DockerHubRegistry>(),
                "nginx",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((
                [new DockerHubTag(1, true, 1, "stable", 1024, 1, 1, "now", "now", "now", DockerHubTagStatus.Active, "tester", [])],
                null));
        github
            .Setup(value => value.GetPackagesAsync(
                It.IsAny<GitHubRegistry>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((
                [new GitHubCrPackage(1, "api", "url", "container", "1", "now", "now", "html")],
                null));
        github
            .Setup(value => value.GetPackageVersionsAsync(
                It.IsAny<GitHubRegistry>(),
                "api",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((
                [new GitHubCrPackageVersion(1, "url", "sha256:1", "html", "now", "now", "package", null)],
                null));
        connector
            .Setup(value => value.InspectImageAsync(
                It.Is<InspectImageCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.ImageId == SeededDockerImageId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectResult(SeededDockerImageId)));
        connector
            .Setup(value => value.GetExposedPortsAsync(
                It.Is<RunImageInfoCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.ImageId == SeededDockerImageId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ExposedPortsResult(["80/tcp"])));

        using var listResponse = await Client.GetAsync($"/api/v1/images/{PlatformId:D}", cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var list = JsonDocument.Parse(await listResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Equal(SeededDockerImageId, list.RootElement.GetProperty("images")[0].GetProperty("dockerImageId").GetString());

        using var repositoriesResponse = await Client.GetAsync(
            "/api/v1/images/dockerhub/docker-endpoint/repositories",
            cancellationToken);
        repositoriesResponse.EnsureSuccessStatusCode();
        Assert.Contains("nginx", await repositoriesResponse.Content.ReadAsStringAsync(cancellationToken));

        using var externalResponse = await Client.GetAsync(
            "/api/v1/images/github-endpoint/repositories",
            cancellationToken);
        externalResponse.EnsureSuccessStatusCode();
        Assert.Contains("api", await externalResponse.Content.ReadAsStringAsync(cancellationToken));

        using var tagsResponse = await Client.GetAsync(
            "/api/v1/images/dockerhub/docker-endpoint/nginx/tags",
            cancellationToken);
        tagsResponse.EnsureSuccessStatusCode();
        Assert.Contains("stable", await tagsResponse.Content.ReadAsStringAsync(cancellationToken));

        using var versionsResponse = await Client.GetAsync(
            "/api/v1/images/ghcr/github-endpoint/api/versions",
            cancellationToken);
        versionsResponse.EnsureSuccessStatusCode();

        using var inspectResponse = await Client.GetAsync(
            $"/api/v1/images/{PlatformId:D}/{SeededDockerImageId}",
            cancellationToken);
        inspectResponse.EnsureSuccessStatusCode();
        Assert.Contains("nginx", await inspectResponse.Content.ReadAsStringAsync(cancellationToken));

        using var portsResponse = await Client.GetAsync(
            $"/api/v1/images/{PlatformId:D}/{imageId:D}/_ports",
            cancellationToken);
        portsResponse.EnsureSuccessStatusCode();
        Assert.Contains("80/tcp", await portsResponse.Content.ReadAsStringAsync(cancellationToken));

        dockerHub.VerifyAll();
        github.VerifyAll();
    }

    [Fact]
    public async Task PullImageEndpoint_ShouldStreamAndPersistPulledImage()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        connector
            .Setup(value => value.PullImageProgressStreamAsync(
                It.Is<PullImageCommand>(command =>
                    command.PlatformAddress == "docker.test:2375"
                    && command.FromImage == "registry-1.docker.io/tester/nginx:latest"),
                It.IsAny<CancellationToken>()))
            .Returns(PullProgress());
        connector
            .Setup(value => value.GetAsync(
                "docker.test:2375",
                "registry-1.docker.io/tester/nginx:latest",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ImageResult(
                PulledDockerImageId,
                2048,
                0,
                string.Empty,
                0,
                DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                2048,
                ["registry-1.docker.io/tester/nginx:latest"],
                [$"registry-1.docker.io/tester/nginx@{PulledDockerImageId}"],
                new Dictionary<string, string>())));

        using var response = await Client.PostAsJsonAsync(
            "/api/v1/images/pull",
            new
            {
                platformId = PlatformId,
                registryId = dockerRegistryId,
                imageTag = "nginx"
            },
            cancellationToken);
        response.EnsureSuccessStatusCode();
        Assert.Contains(PulledDockerImageId, await response.Content.ReadAsStringAsync(cancellationToken));

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.Images.GetByDockerImageIdAsync(
            PulledDockerImageId,
            PlatformId,
            cancellationToken);
        Assert.NotNull(persisted);
        Assert.Contains("registry-1.docker.io/tester/nginx:latest", persisted.Tags);

        connector.VerifyAll();
    }

    private static InspectImageResult InspectResult(string dockerImageId)
        => new(
            dockerImageId,
            1024,
            "linux",
            "2026-08-04T00:00:00Z",
            "amd64",
            ["ENV=test"],
            ["nginx"],
            ["nginx:stable"],
            [],
            ["80/tcp"],
            [],
            new Dictionary<string, string>(),
            []);

    private static async IAsyncEnumerable<PullImageStreamItem> PullProgress(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        yield return new PullImageStreamItem(Status: "Pulling");
        await Task.CompletedTask;
    }

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => channel.Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            await item.ExecuteAsync(scope.ServiceProvider.GetRequiredService<IUnitOfWork>(), cancellationToken);
        }
    }
}
