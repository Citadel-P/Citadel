using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Threading.Channels;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class SwarmEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<ISwarmConnector> connector = new(MockBehavior.Strict);
    private readonly Mock<INetworkConnector> networkConnector = new(MockBehavior.Strict);
    private Guid platformId;
    private Guid taskContainerId;
    private Guid otherPlatformId;
    private Guid standalonePlatformId;
    private bool secretDeleteReportsNotFound;
    private readonly List<SwarmNodeResult> liveNodes =
    [
        new("primary-node", 1, "primary-manager", "Manager", true, "Reachable", "Ready", null,
            "Active", "28.0", "linux", "x86_64", "10.0.0.1",
            new Dictionary<string, string> { ["zone"] = "primary" }, 1, 1, null, null)
    ];
    private readonly List<SwarmSecretResult> liveSecrets =
    [
        new("primary-secret", 3, "primary-secret-name", null,
            new Dictionary<string, string> { ["secret"] = "primary" }, null, null),
        new("primary-unused-secret", 1, "primary-unused-secret", null,
            new Dictionary<string, string>(), null, null)
    ];
    private readonly List<SwarmConfigResult> liveConfigs =
    [
        new("primary-config", 4, "primary-config-name", "golang",
            new Dictionary<string, string> { ["config"] = "primary" }, null, null),
        new("primary-unused-config", 1, "primary-unused-config", null,
            new Dictionary<string, string>(), null, null)
    ];
    private readonly List<SwarmNetworkResult> liveNetworks =
    [
        new("primarynetwork", "primary-overlay", "Swarm", "overlay", true, false, false, true,
            false, ["10.0.0.0/24"], new Dictionary<string, string> { ["network"] = "primary" }, null)
    ];

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Result.Success<IReadOnlyList<SwarmNodeResult>>(liveNodes.ToArray()));
        connector
            .Setup(value => value.UpdateNodeAsync(It.IsAny<UpdateSwarmNodeCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((UpdateSwarmNodeCommand command, CancellationToken _) =>
            {
                var index = liveNodes.FindIndex(value => value.Id == command.NodeId);
                var current = liveNodes[index];
                liveNodes[index] = current with
                {
                    VersionIndex = current.VersionIndex + 1,
                    Availability = command.Availability,
                    Labels = command.Labels
                };
                return Result.Success();
            });
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmServiceResult>>(
            [
                new("primary-service", 2, "primary-web", "Replicated", "nginx:primary", 1, 1,
                    "Completed", null, ["80:80/tcp"], ["primarynetwork"], ["primary-secret"],
                    ["primary-config"], new Dictionary<string, string> { ["app"] = "primary" }, null, null)
            ]));
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>(
            [
                new("primary-task-1", 1, "primary-web.1", "primary-service", 1, "primary-node",
                    "Running", "Running", null, null, "nginx:primary", ["80/tcp"], null, null, null,
                    "container-primary-task-1")
            ]));
        connector
            .Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Result.Success<IReadOnlyList<SwarmNetworkResult>>(liveNetworks.ToArray()));
        networkConnector
            .Setup(value => value.CreateNetworkAsync(It.IsAny<CreateDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((CreateDockerNetworkCommand command, CancellationToken _) =>
            {
                var id = $"network{command.Name.Replace("-", string.Empty, StringComparison.Ordinal)}";
                liveNetworks.Add(new SwarmNetworkResult(
                    id, command.Name, "Swarm", command.Driver ?? "overlay", command.Attachable ?? false,
                    command.Internal ?? false, command.Ingress ?? false, false,
                    command.EnableIPv6 ?? false,
                    (command.Ipam?.Config ?? []).Select(value => value.Subnet).OfType<string>().Where(value => !string.IsNullOrWhiteSpace(value)).ToArray(),
                    command.Labels, null));
                return Result.Success(new CreateDockerNetworkResult(id));
            });
        networkConnector
            .Setup(value => value.InspectNetworkAsync(It.IsAny<InspectNetworkCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectNetworkCommand command, CancellationToken _) =>
            {
                var network = liveNetworks.Single(value => value.Id == command.NetworkId);
                return Result.Success(new DockerNetworkDetails(
                    network.Name, network.Id, string.Empty, network.Driver, network.Scope,
                    true, network.EnableIPv6, network.IsInternal, network.IsAttachable,
                    network.IsIngress, false, null, null, new Dictionary<string, string>(),
                    network.Labels, new Dictionary<string, NetworkConnectedContainer>(), []));
            });
        networkConnector
            .Setup(value => value.DeleteNetworkAsync(It.IsAny<DeleteDockerNetworkCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((DeleteDockerNetworkCommand command, CancellationToken _) =>
            {
                foreach (var id in command.Ids)
                    liveNetworks.RemoveAll(value => value.Id == id);
                return Result.Success();
            });
        connector
            .Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Result.Success<IReadOnlyList<SwarmSecretResult>>(liveSecrets.ToArray()));
        connector
            .Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Result.Success<IReadOnlyList<SwarmConfigResult>>(liveConfigs.ToArray()));
        connector
            .Setup(value => value.GetConfigDataAsync(It.IsAny<InspectSwarmConfigCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("setting=true\n"u8.ToArray()));
        connector
            .Setup(value => value.CreateSecretAsync(It.IsAny<CreateSwarmSecretCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((CreateSwarmSecretCommand command, CancellationToken _) =>
            {
                var created = new SwarmSecretResult(
                    $"secret-{command.Name}", 1, command.Name, null, command.Labels, null, null);
                liveSecrets.Add(created);
                return Result.Success();
            });
        connector
            .Setup(value => value.UpdateSecretLabelsAsync(It.IsAny<UpdateSwarmSecretLabelsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((UpdateSwarmSecretLabelsCommand command, CancellationToken _) =>
            {
                var index = liveSecrets.FindIndex(value => value.Id == command.SecretId);
                var current = liveSecrets[index];
                var updated = current with { VersionIndex = current.VersionIndex + 1, Labels = command.Labels };
                liveSecrets[index] = updated;
                return Result.Success();
            });
        connector
            .Setup(value => value.DeleteSecretAsync(It.IsAny<DeleteSwarmSecretCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((DeleteSwarmSecretCommand command, CancellationToken _) =>
            {
                if (secretDeleteReportsNotFound)
                    return Result.Failure(new NotFoundError("Secret does not exist."));

                liveSecrets.RemoveAll(value => value.Id == command.SecretId);
                return Result.Success();
            });
        connector
            .Setup(value => value.CreateConfigAsync(It.IsAny<CreateSwarmConfigCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((CreateSwarmConfigCommand command, CancellationToken _) =>
            {
                var created = new SwarmConfigResult(
                    $"config-{command.Name}", 1, command.Name, null, command.Labels, null, null);
                liveConfigs.Add(created);
                return Result.Success();
            });
        connector
            .Setup(value => value.UpdateConfigLabelsAsync(It.IsAny<UpdateSwarmConfigLabelsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((UpdateSwarmConfigLabelsCommand command, CancellationToken _) =>
            {
                var index = liveConfigs.FindIndex(value => value.Id == command.ConfigId);
                var current = liveConfigs[index];
                var updated = current with { VersionIndex = current.VersionIndex + 1, Labels = command.Labels };
                liveConfigs[index] = updated;
                return Result.Success();
            });
        connector
            .Setup(value => value.DeleteConfigAsync(It.IsAny<DeleteSwarmConfigCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((DeleteSwarmConfigCommand command, CancellationToken _) =>
            {
                liveConfigs.RemoveAll(value => value.Id == command.ConfigId);
                return Result.Success();
            });
        connector
            .Setup(value => value.InspectNodeAsync(
                It.IsAny<InspectSwarmNodeCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectSwarmNodeCommand command, CancellationToken _) => Result.Success(
                new SwarmNodeResult(
                    command.NodeId,
                    2,
                    "primary-manager-live",
                    "Manager",
                    true,
                    "Reachable",
                    "Ready",
                    null,
                    "Active",
                    "29.0",
                    "linux",
                    "x86_64",
                    "10.0.0.10",
                    new Dictionary<string, string> { ["zone"] = "primary" },
                    3,
                    3,
                    null,
                    null)));
        connector
            .Setup(value => value.InspectServiceAsync(
                It.IsAny<InspectSwarmServiceCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectSwarmServiceCommand command, CancellationToken _) => Result.Success(
                new SwarmServiceResult(
                    command.ServiceId,
                    2,
                    "primary-web",
                    "Replicated",
                    "nginx:inspected",
                    2,
                    2,
                    "Completed",
                    null,
                    ["80/tcp"],
                    ["primarynetwork"],
                    ["primary-secret"],
                    ["primary-config"],
                    new Dictionary<string, string> { ["environment"] = "test" },
                    null,
                    null)));
        connector
            .Setup(value => value.GetServiceLogsAsync(
                It.IsAny<GetSwarmServiceLogsCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new SwarmLogsResult(["service output"], false)));
        connector
            .Setup(value => value.GetTaskLogsAsync(
                It.IsAny<GetSwarmTaskLogsCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new SwarmLogsResult(["task output"], true)));
        connector
            .Setup(value => value.InspectTaskAsync(
                It.IsAny<InspectSwarmTaskCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectSwarmTaskCommand command, CancellationToken _) => Result.Success(
                new SwarmTaskResult(
                    command.TaskId,
                    1,
                    "primary-web.1",
                    "primary-service",
                    1,
                    command.TaskId == "primary-task-2" ? "worker-node" : "node-swarm-endpoints",
                    "Running",
                    "Running",
                    null,
                    null,
                    "nginx:primary",
                    ["80/tcp"],
                    null,
                    null,
                    null,
                    ContainerId: $"container-{command.TaskId}")));
        services.ReplaceService<IConnectorFactory<ISwarmConnector>>(
            new FakeConnectorFactory(connector.Object));
        services.ReplaceService<IConnectorFactory<INetworkConnector>>(
            new FakeNetworkConnectorFactory(networkConnector.Object));
        var platformCache = new Mock<IPlatformContainerCache>();
        var cacheEntry = new PlatformCacheEntry(
            Guid.Empty,
            "https://swarm-endpoints.test",
            PlatformConnectorType.Agent,
            []);
        Error? cacheError = null;
        platformCache
            .Setup(value => value.TryGetCacheEntry(
                It.IsAny<Guid>(), out cacheEntry, out cacheError))
            .Returns(true);
        services.ReplaceService<IPlatformContainerCache>(platformCache.Object);
        services.RemoveService<IDbWorkQueue>();
        services.AddSingleton<IDbWorkQueue, InlineDbWorkQueue>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreateSwarmPlatform("swarm-endpoints", "cluster-endpoints");
        var otherPlatform = CreateSwarmPlatform("other-swarm", "other-cluster");
        var standalonePlatform = CreateStandalonePlatform();

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(standalonePlatform, TestContext.Current.CancellationToken);
        var taskContainer = new Container(
            "primary-web.1",
            "sha256:image",
            platform.Id,
            "container-primary-task-1",
            ContainerStateStatus.Running);
        await uow.Containers.AddAsync(taskContainer, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        await uow.ContainerStats.BulkInsertAsync(
            [new ContainerStat(taskContainer.Id, 256, 32, 12.5, 1024, 2048, 1024,
                DateTimeOffset.UtcNow.ToUnixTimeSeconds())],
            TestContext.Current.CancellationToken);
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
        taskContainerId = taskContainer.Id;
        otherPlatformId = otherPlatform.Id;
        standalonePlatformId = standalonePlatform.Id;
    }

    [Theory]
    [InlineData("nodes", "primary-node", "hostname", "primary-manager")]
    [InlineData("services", "primary-service", "image", "nginx:primary")]
    [InlineData("tasks", "primary-task-1", "nodeHostname", "primary-manager")]
    [InlineData("networks", "primarynetwork", "driver", "overlay")]
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

    [Theory]
    [InlineData("nodes", "primary-node")]
    [InlineData("services", "primary-service")]
    [InlineData("tasks", "primary-task-1")]
    [InlineData("networks", "primarynetwork")]
    [InlineData("secrets", "primary-secret")]
    [InlineData("configs", "primary-config")]
    public async Task ListAndDetailEndpoints_ShouldReturnCallerPlatformCapabilities(
        string resource,
        string resourceId)
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Logs | SpecificPermission.Inspect)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var listResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}",
            TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        using (var listDocument = await ReadJsonAsync(listResponse))
        {
            AssertReadInspectAndLogsCapabilities(listDocument.RootElement.GetProperty("capabilities"));
            var item = listDocument.RootElement
                .GetProperty("items")
                .EnumerateArray()
                .Single(value => value.GetProperty("id").GetString() == resourceId);
            AssertReadInspectAndLogsCapabilities(item.GetProperty("capabilities"));
        }

        var detailResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}/{resourceId}",
            TestContext.Current.CancellationToken);
        detailResponse.EnsureSuccessStatusCode();

        using var detailDocument = await ReadJsonAsync(detailResponse);
        AssertReadInspectAndLogsCapabilities(detailDocument.RootElement.GetProperty("capabilities"));
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

    [Fact]
    public async Task NodeUpdate_ShouldPersistReconciledAvailabilityAndLabels()
    {
        var response = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node",
            JsonContent("""{"versionIndex":1,"availability":"Drain","labels":{"zone":"maintenance"}}"""),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
        connector.Verify(value => value.UpdateNodeAsync(
            It.Is<UpdateSwarmNodeCommand>(command =>
                command.PlatformAddress == "https://swarm-endpoints.test"
                && command.NodeId == "primary-node"
                && command.VersionIndex == 1
                && command.Availability == "Drain"
                && command.Labels["zone"] == "maintenance"),
            It.IsAny<CancellationToken>()), Times.Once);

        var detail = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node",
            TestContext.Current.CancellationToken);
        detail.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(detail);
        Assert.Equal("Drain", document.RootElement.GetProperty("availability").GetString());
        Assert.Equal("maintenance", document.RootElement.GetProperty("labels").GetProperty("zone").GetString());
        Assert.Equal(2, document.RootElement.GetProperty("versionIndex").GetInt64());
    }

    [Fact]
    public async Task NodeUpdate_ShouldRequireWriteAccessAndCurrentVersion()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer", CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node",
            JsonContent("""{"versionIndex":1,"availability":"Pause","labels":{}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);

        var deniedBulk = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/availability",
            JsonContent("""{"availability":"Pause","nodes":[{"nodeId":"primary-node","versionIndex":1}]}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, deniedBulk.StatusCode);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
        var stale = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node",
            JsonContent("""{"versionIndex":99,"availability":"Pause","labels":{}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Conflict, stale.StatusCode);
    }

    [Fact]
    public async Task NodeAvailabilityUpdate_ShouldUpdateAllSelectedNodesAndPersistReconciliation()
    {
        var worker = new SwarmNodeResult(
            "worker-node", 4, "worker-1", "Worker", false, "", "Ready", null,
            "Active", "28.0", "linux", "x86_64", "10.0.0.2",
            new Dictionary<string, string> { ["zone"] = "worker" }, 0, 0, null, null);
        liveNodes.Add(worker);

        var snapshot = CreateSnapshot(platformId, "primary", taskCount: 3);
        var observedAt = snapshot.Nodes[0].ObservedAt;
        var workerProjection = new SwarmNodeProjection(
            platformId, worker.Id, worker.VersionIndex, worker.Hostname, worker.Role, worker.IsLeader,
            worker.Reachability, worker.Status, worker.StatusMessage, worker.Availability,
            worker.EngineVersion, worker.OperatingSystem, worker.Architecture, worker.Address,
            worker.Labels, worker.RunningTaskCount, worker.DesiredTaskCount,
            worker.CreatedAt, worker.UpdatedAt, observedAt, false);
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await unitOfWork.Swarm.ReplaceAsync(
                platformId,
                snapshot with { Nodes = [.. snapshot.Nodes, workerProjection] },
                TestContext.Current.CancellationToken);
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/availability",
            JsonContent("""
                {
                  "availability":"Pause",
                  "nodes":[
                    {"nodeId":"primary-node","versionIndex":1},
                    {"nodeId":"worker-node","versionIndex":4}
                  ]
                }
                """),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
        connector.Verify(value => value.UpdateNodeAsync(
            It.Is<UpdateSwarmNodeCommand>(command =>
                command.NodeId == "primary-node"
                && command.Availability == "Pause"
                && command.Labels["zone"] == "primary"),
            It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.UpdateNodeAsync(
            It.Is<UpdateSwarmNodeCommand>(command =>
                command.NodeId == "worker-node"
                && command.Availability == "Pause"
                && command.Labels["zone"] == "worker"),
            It.IsAny<CancellationToken>()), Times.Once);

        var nodes = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes",
            TestContext.Current.CancellationToken);
        nodes.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(nodes);
        Assert.All(
            document.RootElement.GetProperty("items").EnumerateArray(),
            node => Assert.Equal("Pause", node.GetProperty("availability").GetString()));
    }

    [Fact]
    public async Task NodeAvailabilityUpdate_ShouldPreflightEveryNodeBeforeChangingDocker()
    {
        var response = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/availability",
            JsonContent("""
                {
                  "availability":"Drain",
                  "nodes":[
                    {"nodeId":"primary-node","versionIndex":1},
                    {"nodeId":"missing-node","versionIndex":1}
                  ]
                }
                """),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        connector.Verify(
            value => value.UpdateNodeAsync(It.IsAny<UpdateSwarmNodeCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task NodeAvailabilityUpdate_ShouldRejectAnEmptySelection()
    {
        var response = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/availability",
            JsonContent("""{"availability":"Pause"}"""),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
        connector.Verify(
            value => value.UpdateNodeAsync(It.IsAny<UpdateSwarmNodeCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task OverlayNetworkLifecycle_ShouldCreatePersistAndDeleteAnUnusedNetwork()
    {
        var create = await Client.PostAsync(
            "/api/v1/networks/",
            System.Net.Http.Json.JsonContent.Create(new
            {
                PlatformId = platformId,
                Name = "metrics-overlay",
                Driver = "overlay",
                Scope = "swarm",
                EnableIPv4 = true,
                EnableIPv6 = false,
                Internal = false,
                Attachable = true,
                Ingress = false,
                ConfigOnly = false,
                Ipam = new
                {
                    Driver = "default",
                    Config = new[]
                    {
                        new { Subnet = "10.22.0.0/24", IpRange = "", Gateway = "" },
                        new { Subnet = "", IpRange = "", Gateway = "" }
                    }
                },
                Labels = new Dictionary<string, string> { ["team"] = "ops" }
            }),
            TestContext.Current.CancellationToken);
        Assert.True(
            create.IsSuccessStatusCode,
            await create.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));

        var projected = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/networks/networkmetricsoverlay",
            TestContext.Current.CancellationToken);
        projected.EnsureSuccessStatusCode();
        using (var document = await ReadJsonAsync(projected))
        {
            Assert.Equal("metrics-overlay", document.RootElement.GetProperty("name").GetString());
            Assert.Empty(document.RootElement.GetProperty("serviceNames").EnumerateArray());
        }

        var delete = await SendDeleteAsync(
            "/api/v1/networks",
            $$"""{"platformId":"{{platformId}}","ids":["networkmetricsoverlay"]}""");
        Assert.Equal(HttpStatusCode.NoContent, delete.StatusCode);

        var afterDelete = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/networks/networkmetricsoverlay",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, afterDelete.StatusCode);
    }

    [Fact]
    public async Task DeleteOverlayNetwork_ShouldRejectReferencedNetworkBeforeDockerMutation()
    {
        var response = await SendDeleteAsync(
            "/api/v1/networks",
            $$"""{"platformId":"{{platformId}}","ids":["primarynetwork"]}""");

        Assert.True(
            response.StatusCode == HttpStatusCode.Conflict,
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        networkConnector.Verify(value => value.DeleteNetworkAsync(
            It.IsAny<DeleteDockerNetworkCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task DeleteOverlayNetwork_ShouldRejectStackOwnedNetworkBeforeDockerMutation()
    {
        liveNetworks.Add(new SwarmNetworkResult(
            "stacknetwork",
            "external-stack_default",
            "Swarm",
            "overlay",
            true,
            false,
            false,
            false,
            false,
            [],
            new Dictionary<string, string> { ["com.docker.stack.namespace"] = "external-stack" },
            null));

        var response = await SendDeleteAsync(
            "/api/v1/networks",
            $$"""{"platformId":"{{platformId}}","ids":["stacknetwork"]}""");

        Assert.True(
            response.StatusCode == HttpStatusCode.Conflict,
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        networkConnector.Verify(value => value.DeleteNetworkAsync(
            It.IsAny<DeleteDockerNetworkCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Theory]
    [InlineData("secrets", "primary-secret", true)]
    [InlineData("secrets", "primary-unused-secret", false)]
    [InlineData("configs", "primary-config", true)]
    [InlineData("configs", "primary-unused-config", false)]
    public async Task SecretAndConfigEndpoints_ShouldExposePersistedUsageState(
        string resource,
        string resourceId,
        bool expectedInUse)
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}/{resourceId}",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        Assert.Equal(expectedInUse, document.RootElement.GetProperty("inUse").GetBoolean());
    }

    [Fact]
    public async Task SecretLifecycle_ShouldPersistProjectionWithoutReturningSecretData()
    {
        var create = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets",
            JsonContent("""{"name":"new-secret","data":"highly-sensitive","labels":{"team":"ops"}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, create.StatusCode);

        var detail = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/secret-new-secret",
            TestContext.Current.CancellationToken);
        detail.EnsureSuccessStatusCode();
        using (var document = await ReadJsonAsync(detail))
        {
            Assert.False(document.RootElement.GetProperty("inUse").GetBoolean());
            Assert.Equal("ops", document.RootElement.GetProperty("labels").GetProperty("team").GetString());
            Assert.False(document.RootElement.TryGetProperty("data", out _));
            Assert.False(document.RootElement.TryGetProperty("value", out _));
        }
        connector.Verify(value => value.CreateSecretAsync(
            It.Is<CreateSwarmSecretCommand>(command =>
                command.Name == "new-secret" && Encoding.UTF8.GetString(command.Data) == "highly-sensitive"),
            It.IsAny<CancellationToken>()), Times.Once);

        var update = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/secret-new-secret/labels",
            JsonContent("""{"versionIndex":1,"labels":{"team":"platform"}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, update.StatusCode);

        var updatedDetail = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/secret-new-secret",
            TestContext.Current.CancellationToken);
        updatedDetail.EnsureSuccessStatusCode();
        using (var document = await ReadJsonAsync(updatedDetail))
        {
            Assert.Equal("platform", document.RootElement.GetProperty("labels").GetProperty("team").GetString());
            Assert.Equal(2, document.RootElement.GetProperty("versionIndex").GetInt64());
        }

        var delete = await SendDeleteAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets",
            """{"ids":["secret-new-secret"]}""");
        Assert.Equal(HttpStatusCode.NoContent, delete.StatusCode);

        var afterDelete = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/secret-new-secret",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, afterDelete.StatusCode);
    }

    [Fact]
    public async Task ConfigLifecycle_ShouldPersistCreateLabelUpdateAndDelete()
    {
        var create = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs",
            JsonContent("""{"name":"new-config","data":"setting=true","labels":{}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, create.StatusCode);

        var update = await Client.PatchAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/config-new-config/labels",
            JsonContent("""{"versionIndex":1,"labels":{"environment":"test"}}"""),
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, update.StatusCode);

        var detail = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/config-new-config",
            TestContext.Current.CancellationToken);
        detail.EnsureSuccessStatusCode();
        using (var document = await ReadJsonAsync(detail))
        {
            Assert.False(document.RootElement.GetProperty("inUse").GetBoolean());
            Assert.Equal("test", document.RootElement.GetProperty("labels").GetProperty("environment").GetString());
        }

        var delete = await SendDeleteAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs",
            """{"ids":["config-new-config"]}""");
        Assert.Equal(HttpStatusCode.NoContent, delete.StatusCode);
        connector.Verify(value => value.DeleteConfigAsync(
            It.Is<DeleteSwarmConfigCommand>(command => command.ConfigId == "config-new-config"),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Theory]
    [InlineData("secrets", "primary-secret")]
    [InlineData("configs", "primary-config")]
    public async Task DeleteReferencedResource_ShouldReturnConflictWithoutCallingDocker(string resource, string resourceId)
    {
        var response = await SendDeleteAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}",
            $$"""{"ids":["{{resourceId}}"]}""");

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        connector.Verify(value => value.DeleteSecretAsync(
            It.IsAny<DeleteSwarmSecretCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        connector.Verify(value => value.DeleteConfigAsync(
            It.IsAny<DeleteSwarmConfigCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task DeleteSecret_ShouldSucceedWhenDockerReportsItWasAlreadyRemoved()
    {
        liveSecrets.RemoveAll(value => value.Id == "primary-unused-secret");
        secretDeleteReportsNotFound = true;

        var response = await SendDeleteAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets",
            """{"ids":["primary-unused-secret"]}""");

        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
        var detail = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/secrets/primary-unused-secret",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, detail.StatusCode);
    }

    [Theory]
    [InlineData("secrets", "{\"name\":\"denied\",\"data\":\"value\"}")]
    [InlineData("configs", "{\"name\":\"denied\",\"data\":\"value\"}")]
    public async Task MutationEndpoints_ShouldRequireWriteAccess(string resource, string payload)
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer", CreateJwtToken(reader.UserId, reader.ActorId));

        var response = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}",
            JsonContent(payload),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
    }

    [Fact]
    public async Task OverviewEndpoint_ShouldReturnPersistedClusterSummary()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        var root = document.RootElement;
        Assert.Equal("Healthy", root.GetProperty("health").GetString());
        Assert.False(root.GetProperty("isStale").GetBoolean());
        Assert.False(root.TryGetProperty("observedAt", out _));
        Assert.Equal(1, root.GetProperty("nodeCount").GetInt32());
        Assert.Equal(1, root.GetProperty("managerCount").GetInt32());
        Assert.Equal(1, root.GetProperty("serviceCount").GetInt32());
        var serviceStatusCounts = root.GetProperty("serviceStatusCounts");
        Assert.Equal(1, serviceStatusCounts.GetProperty("total").GetInt32());
        Assert.Equal(1, serviceStatusCounts.GetProperty("healthy").GetInt32());
        Assert.Equal(0, serviceStatusCounts.GetProperty("degraded").GetInt32());
        Assert.Equal(0, serviceStatusCounts.GetProperty("failed").GetInt32());
        Assert.Equal(0, serviceStatusCounts.GetProperty("stopped").GetInt32());
        Assert.Equal(0, serviceStatusCounts.GetProperty("unknown").GetInt32());
        Assert.Equal(3, root.GetProperty("runningTaskCount").GetInt32());
        Assert.Equal(3, root.GetProperty("desiredTaskCount").GetInt32());
        Assert.Equal(1, root.GetProperty("networkCount").GetInt32());
        Assert.False(root.TryGetProperty("inventory", out _));
        Assert.True(root.GetProperty("capabilities").GetProperty("canRead").GetBoolean());
    }

    [Fact]
    public async Task ServiceEndpoint_ShouldExposePersistedOwnershipClassification()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/primary-service",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        var root = document.RootElement;
        Assert.Equal("DockerStackExternal", root.GetProperty("ownership").GetString());
        Assert.Equal("primary-stack", root.GetProperty("dockerStackNamespace").GetString());
        Assert.Equal("Orphaned Citadel metadata", root.GetProperty("ownershipDiagnostic").GetString());
    }

    [Theory]
    [InlineData("services", "primary-service", 0)]
    [InlineData("services", "primary-service", 201)]
    [InlineData("tasks", "primary-task-1", 0)]
    [InlineData("tasks", "primary-task-1", 201)]
    public async Task LogsEndpoint_ShouldRejectOutOfRangeTail(string resource, string resourceId, int tail)
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/{resource}/{resourceId}/logs?tail={tail}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task LogsEndpoint_ShouldRequirePlatformLogsPermission()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/primary-service/logs",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
        connector.Verify(
            value => value.GetServiceLogsAsync(
                It.IsAny<GetSwarmServiceLogsCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task LogsEndpoints_ShouldReturnBoundedConnectorResultsForProjectedResources()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Logs)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var serviceResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/primary-service/logs?tail=25",
            TestContext.Current.CancellationToken);
        serviceResponse.EnsureSuccessStatusCode();
        using (var serviceDocument = await ReadJsonAsync(serviceResponse))
        {
            Assert.Equal("service output", serviceDocument.RootElement.GetProperty("lines")[0].GetString());
            Assert.False(serviceDocument.RootElement.GetProperty("truncated").GetBoolean());
        }

        var taskResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/logs?tail=12",
            TestContext.Current.CancellationToken);
        taskResponse.EnsureSuccessStatusCode();
        using (var taskDocument = await ReadJsonAsync(taskResponse))
        {
            Assert.Equal("task output", taskDocument.RootElement.GetProperty("lines")[0].GetString());
            Assert.True(taskDocument.RootElement.GetProperty("truncated").GetBoolean());
        }

        connector.Verify(value => value.GetServiceLogsAsync(
            It.Is<GetSwarmServiceLogsCommand>(command => command.ServiceId == "primary-service" && command.Tail == 25),
            It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.GetTaskLogsAsync(
            It.Is<GetSwarmTaskLogsCommand>(command => command.TaskId == "primary-task-1" && command.Tail == 12),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task LogsEndpoint_ShouldRejectAnIdOutsideTheSelectedPlatformBeforeCallingConnector()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/foreign-service/logs",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        connector.Verify(
            value => value.GetServiceLogsAsync(
                It.IsAny<GetSwarmServiceLogsCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task TaskInspectEndpoint_ShouldRequireInspectPermissionAndReturnLiveTaskData()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/inspect",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);
        connector.Verify(
            value => value.InspectTaskAsync(It.IsAny<InspectSwarmTaskCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var inspector = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Inspect)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(inspector.UserId, inspector.ActorId));

        var allowed = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/inspect",
            TestContext.Current.CancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(allowed);
        Assert.Equal("primary-task-1", document.RootElement.GetProperty("id").GetString());
        Assert.Equal("container-primary-task-1", document.RootElement.GetProperty("containerId").GetString());
    }

    [Fact]
    public async Task ServiceInspectEndpoint_ShouldRequireInspectPermissionAndReturnLiveServiceData()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/primary-service/inspect",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);
        connector.Verify(
            value => value.InspectServiceAsync(It.IsAny<InspectSwarmServiceCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var inspector = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Inspect)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(inspector.UserId, inspector.ActorId));

        var allowed = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/services/primary-service/inspect",
            TestContext.Current.CancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(allowed);
        Assert.Equal("primary-service", document.RootElement.GetProperty("id").GetString());
        Assert.Equal("nginx:inspected", document.RootElement.GetProperty("image").GetString());
        Assert.Equal("test", document.RootElement.GetProperty("labels").GetProperty("environment").GetString());
    }

    [Fact]
    public async Task ConfigDataEndpoint_ShouldRequireInspectPermissionAndReturnLiveUtf8Content()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/primary-config/content",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);
        connector.Verify(
            value => value.GetConfigDataAsync(It.IsAny<InspectSwarmConfigCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var inspector = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Inspect)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(inspector.UserId, inspector.ActorId));

        var allowed = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/primary-config/content",
            TestContext.Current.CancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(allowed);
        Assert.Equal("setting=true\n", document.RootElement.GetProperty("content").GetString());
        connector.Verify(
            value => value.GetConfigDataAsync(
                It.Is<InspectSwarmConfigCommand>(command =>
                    command.ConfigId == "primary-config"
                    && command.PlatformAddress == "https://swarm-endpoints.test"),
                It.IsAny<CancellationToken>()),
            Times.Once);

        var unknown = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/not-in-inventory/content",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, unknown.StatusCode);
        connector.Verify(
            value => value.GetConfigDataAsync(
                It.Is<InspectSwarmConfigCommand>(command => command.ConfigId == "not-in-inventory"),
                It.IsAny<CancellationToken>()),
            Times.Never);

        connector
            .Setup(value => value.GetConfigDataAsync(
                It.Is<InspectSwarmConfigCommand>(command => command.ConfigId == "primary-config"),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<byte[]>([0xC3, 0x28]));
        var binary = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/configs/primary-config/content",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.BadRequest, binary.StatusCode);
    }

    [Fact]
    public async Task NodeInspectEndpoint_ShouldRequireInspectPermissionAndReturnLiveNodeData()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node/inspect",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);
        connector.Verify(
            value => value.InspectNodeAsync(It.IsAny<InspectSwarmNodeCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        var inspector = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Inspect)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(inspector.UserId, inspector.ActorId));

        var allowed = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes/primary-node/inspect",
            TestContext.Current.CancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(allowed);
        Assert.Equal("primary-node", document.RootElement.GetProperty("id").GetString());
        Assert.Equal("primary-manager-live", document.RootElement.GetProperty("hostname").GetString());
        Assert.Equal("primary", document.RootElement.GetProperty("labels").GetProperty("zone").GetString());
    }

    [Fact]
    public async Task TaskStatsEndpoint_ShouldUseTheRunningContainerOnTheConnectedNode()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/stats",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await ReadJsonAsync(response);
        Assert.Equal(
            "container-primary-task-1",
            document.RootElement.GetProperty("dockerContainerId").GetString());
        var stats = document.RootElement.GetProperty("stats").EnumerateArray().Single();
        Assert.Equal(taskContainerId, stats.GetProperty("containerId").GetGuid());
        Assert.Equal(12.5, stats.GetProperty("cpuUsage").GetDouble());
        Assert.Equal(256, stats.GetProperty("memoryActive").GetDouble());
    }

    [Fact]
    public async Task TaskStatsEndpoint_ShouldExplainTheNodeLocalDockerLimitation()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-2/stats",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
        using var document = await ReadJsonAsync(response);
        Assert.Contains("not running on the connected manager", document.RootElement.GetProperty("detail").GetString());
    }

    [Fact]
    public async Task TaskTerminalEndpoint_ShouldRequireTerminalPermissionAndResolveTheLocalContainer()
    {
        var reader = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(reader.UserId, reader.ActorId));

        var denied = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/terminal",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, denied.StatusCode);

        var terminalUser = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Terminal)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(terminalUser.UserId, terminalUser.ActorId));

        var allowed = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/terminal",
            TestContext.Current.CancellationToken);
        allowed.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(allowed);
        Assert.Equal(
            "container-primary-task-1",
            document.RootElement.GetProperty("dockerContainerId").GetString());

        var remote = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-2/terminal",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Conflict, remote.StatusCode);
    }

    [Fact]
    public async Task TaskStatsEndpoint_ShouldRejectUnsupportedHistoryWindow()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/tasks/primary-task-1/stats?hours=12",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response) =>
        await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

    private static StringContent JsonContent(string json) => new(json, Encoding.UTF8, "application/json");

    private async Task<HttpResponseMessage> SendDeleteAsync(string requestUri, string json)
    {
        using var request = new HttpRequestMessage(HttpMethod.Delete, requestUri)
        {
            Content = JsonContent(json)
        };
        return await Client.SendAsync(request, TestContext.Current.CancellationToken);
    }

    private static void AssertReadInspectAndLogsCapabilities(JsonElement capabilities)
    {
        var serialized = capabilities.GetRawText();
        Assert.True(capabilities.GetProperty("canRead").GetBoolean(), serialized);
        Assert.False(capabilities.GetProperty("canWrite").GetBoolean());
        Assert.False(capabilities.GetProperty("canExecute").GetBoolean());
        Assert.True(capabilities.GetProperty("canViewLogs").GetBoolean(), serialized);
        Assert.True(capabilities.GetProperty("canInspect").GetBoolean(), serialized);
        Assert.False(capabilities.GetProperty("canOpenTerminal").GetBoolean());
        Assert.False(capabilities.GetProperty("canPull").GetBoolean());
    }

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
            taskCount, taskCount, "Completed", null, ["80:80/tcp"], [$"{prefix}network"],
            [$"{prefix}-secret"], [$"{prefix}-config"], new Dictionary<string, string> { ["app"] = prefix },
            observedAt.AddDays(-1), observedAt, observedAt, false,
            SwarmServiceOwnership.DockerStackExternal,
            DockerStackNamespace: $"{prefix}-stack",
            OwnershipDiagnostic: "Orphaned Citadel metadata");
        var tasks = Enumerable.Range(1, taskCount)
            .Select(index => new SwarmTaskProjection(
                selectedPlatformId, $"{prefix}-task-{index}", index, $"{prefix}-web.{index}",
                service.DockerServiceId, service.Name, index, node.DockerNodeId, node.Hostname,
                "Running", "Running", null, null, service.Image, ["80/tcp"],
                observedAt.AddMinutes(index), observedAt.AddDays(-1), observedAt,
                observedAt, false))
            .ToArray();
        var network = new SwarmNetworkProjection(
            selectedPlatformId, $"{prefix}network", $"{prefix}-overlay", "Swarm", "overlay",
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
        var unusedSecret = new SwarmSecretProjection(
            selectedPlatformId, $"{prefix}-unused-secret", 1, $"{prefix}-unused-secret", null,
            [], new Dictionary<string, string>(), observedAt, observedAt, observedAt, false);
        var unusedConfig = new SwarmConfigProjection(
            selectedPlatformId, $"{prefix}-unused-config", 1, $"{prefix}-unused-config", null,
            [], new Dictionary<string, string>(), observedAt, observedAt, observedAt, false);

        return new SwarmProjectionSnapshot([node], [service], tasks, [network], [secret, unusedSecret], [config, unusedConfig]);
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

    private sealed class FakeConnectorFactory(ISwarmConnector value) : IConnectorFactory<ISwarmConnector>
    {
        public ISwarmConnector GetConnector(PlatformConnectorType type) => value;
    }

    private sealed class FakeNetworkConnectorFactory(INetworkConnector value) : IConnectorFactory<INetworkConnector>
    {
        public INetworkConnector GetConnector(PlatformConnectorType type) => value;
    }

    private sealed class InlineDbWorkQueue(IServiceScopeFactory scopeFactory) : IDbWorkQueue
    {
        private readonly Channel<IDbWorkItem> channel = Channel.CreateUnbounded<IDbWorkItem>();

        public ChannelReader<IDbWorkItem> Reader => channel.Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken) =>
            EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            await item.ExecuteAsync(scope.ServiceProvider.GetRequiredService<IUnitOfWork>(), cancellationToken);
        }
    }
}
