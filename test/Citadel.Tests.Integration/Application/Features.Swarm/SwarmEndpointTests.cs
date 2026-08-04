using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class SwarmEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid platformId;
    private Guid otherPlatformId;
    private Guid standalonePlatformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreateSwarmPlatform("swarm-endpoints", "cluster-endpoints");
        var otherPlatform = CreateSwarmPlatform("other-swarm", "other-cluster");
        var standalonePlatform = CreateStandalonePlatform();

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(standalonePlatform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        await uow.Swarm.ReplaceAsync(
            platform.Id,
            CreateSnapshot(platform.Id, "primary", taskCount: 3),
            TestContext.Current.CancellationToken);
        await uow.Swarm.ReplaceAsync(
            otherPlatform.Id,
            CreateSnapshot(otherPlatform.Id, "foreign"),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        otherPlatformId = otherPlatform.Id;
        standalonePlatformId = standalonePlatform.Id;
    }

    [Theory]
    [InlineData("nodes", "primary-node", "hostname", "primary-manager")]
    [InlineData("services", "primary-service", "image", "nginx:primary")]
    [InlineData("tasks", "primary-task-1", "nodeHostname", "primary-manager")]
    [InlineData("networks", "primary-network", "driver", "overlay")]
    [InlineData("secrets", "primary-secret", "name", "primary-secret-name")]
    [InlineData("configs", "primary-config", "name", "primary-config-name")]
    public async Task ListAndDetailEndpoints_ShouldReturnPersistedProjection(
        string resource,
        string resourceId,
        string property,
        string expectedValue)
    {
        var listResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}",
            TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using (var listDocument = await ReadJsonAsync(listResponse))
        {
            var item = listDocument.RootElement
                .GetProperty("items")
                .EnumerateArray()
                .Single(value => value.GetProperty("id").GetString() == resourceId);
            Assert.Equal(expectedValue, item.GetProperty(property).GetString());
        }

        var detailResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}/{resourceId}",
            TestContext.Current.CancellationToken);
        detailResponse.EnsureSuccessStatusCode();

        using var detailDocument = await ReadJsonAsync(detailResponse);
        Assert.Equal(resourceId, detailDocument.RootElement.GetProperty("id").GetString());
        Assert.Equal(expectedValue, detailDocument.RootElement.GetProperty(property).GetString());
    }

    [Fact]
    public async Task TasksEndpoint_ShouldHonorLimitAndPersistedOrdering()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks?limit=2",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        var items = document.RootElement.GetProperty("items").EnumerateArray().ToArray();

        Assert.Equal(2, items.Length);
        Assert.Equal("primary-task-3", items[0].GetProperty("id").GetString());
        Assert.Equal("primary-task-2", items[1].GetProperty("id").GetString());
    }

    [Theory]
    [InlineData(0)]
    [InlineData(201)]
    public async Task TasksEndpoint_ShouldRejectOutOfRangeLimit(int limit)
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks?limit={limit}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task DetailEndpoint_ShouldNotReturnAnotherPlatformsProjection()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/foreign-service",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Endpoint_ShouldRejectStandalonePlatform()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{standalonePlatformId}/swarm/nodes",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Endpoint_ShouldRequireReadAccessToTheSelectedPlatform()
    {
        var unauthorized = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(unauthorized.UserId, unauthorized.ActorId));

        var deniedResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, deniedResponse.StatusCode);

        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var allowedResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes",
            TestContext.Current.CancellationToken);
        allowedResponse.EnsureSuccessStatusCode();

        var otherPlatformResponse = await Client.GetAsync(
            $"/api/v1/platforms/{otherPlatformId}/swarm/nodes",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, otherPlatformResponse.StatusCode);
    }

    [Fact]
    public async Task SecretEndpoint_ShouldExposeMetadataOnly()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/primary-secret",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        Assert.Equal("primary-secret", document.RootElement.GetProperty("id").GetString());
        Assert.False(document.RootElement.TryGetProperty("data", out _));
        Assert.False(document.RootElement.TryGetProperty("value", out _));
    }

    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response) =>
        await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

    private static SwarmProjectionSnapshot CreateSnapshot(Guid selectedPlatformId, string prefix, int taskCount = 1)
    {
        var observedAt = new DateTimeOffset(2026, 8, 4, 12, 0, 0, TimeSpan.Zero);
        var node = new SwarmNodeProjection(
            selectedPlatformId, $"{prefix}-node", 1, $"{prefix}-manager", "Manager", true,
            "Reachable", "Ready", null, "Active", "28.0", "linux", "x86_64", "10.0.0.1",
            new Dictionary<string, string> { ["zone"] = prefix }, taskCount, taskCount,
            observedAt.AddDays(-1), observedAt, observedAt, false);
        var service = new SwarmServiceProjection(
            selectedPlatformId, $"{prefix}-service", 2, $"{prefix}-web", "Replicated", $"nginx:{prefix}",
            taskCount, taskCount, "Completed", null, ["80:80/tcp"], [$"{prefix}-network"],
            [$"{prefix}-secret"], [$"{prefix}-config"], new Dictionary<string, string> { ["app"] = prefix },
            observedAt.AddDays(-1), observedAt, observedAt, false);
        var tasks = Enumerable.Range(1, taskCount)
            .Select(index => new SwarmTaskProjection(
                selectedPlatformId, $"{prefix}-task-{index}", index, $"{prefix}-web.{index}",
                service.DockerServiceId, service.Name, index, node.DockerNodeId, node.Hostname,
                "Running", "Running", null, null, service.Image, ["80/tcp"],
                observedAt.AddMinutes(index), observedAt.AddDays(-1), observedAt,
                observedAt, false))
            .ToArray();
        var network = new SwarmNetworkProjection(
            selectedPlatformId, $"{prefix}-network", $"{prefix}-overlay", "Swarm", "overlay",
            true, false, false, true, false, ["10.0.0.0/24"], [service.Name],
            new Dictionary<string, string> { ["network"] = prefix }, observedAt, observedAt, false);
        var secret = new SwarmSecretProjection(
            selectedPlatformId, $"{prefix}-secret", 3, $"{prefix}-secret-name", null,
            [service.Name], new Dictionary<string, string> { ["secret"] = prefix },
            observedAt, observedAt, observedAt, false);
        var config = new SwarmConfigProjection(
            selectedPlatformId, $"{prefix}-config", 4, $"{prefix}-config-name", "golang",
            [service.Name], new Dictionary<string, string> { ["config"] = prefix },
            observedAt, observedAt, observedAt, false);

        return new SwarmProjectionSnapshot([node], [service], tasks, [network], [secret], [config]);
    }

    private static Platform CreateSwarmPlatform(string name, string clusterId) => new(
        name,
        $"https://{name}.test",
        networkCount: 1,
        volumeCount: 1,
        imageCount: 1,
        cpuCount: 4,
        memTotal: 1024,
        serverVersion: "28.0",
        agentVersion: "test",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerSwarmPlatformDescriptor(
            NodeID: $"node-{name}",
            NodeAddr: "10.0.0.10",
            LocalNodeState: "Active",
            ControlAvailable: true,
            Nodes: 1,
            Managers: 1,
            DaemonId: $"daemon-{name}",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0,
            ClusterId: clusterId),
        clusterId: clusterId);

    private static Platform CreateStandalonePlatform() => new(
        "standalone-endpoints",
        "https://standalone-endpoints.test",
        networkCount: 0,
        volumeCount: 0,
        imageCount: 0,
        cpuCount: 2,
        memTotal: 512,
        serverVersion: "28.0",
        agentVersion: "test",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerPlatformDescriptor(
            DaemonId: "standalone-daemon",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0));
}
