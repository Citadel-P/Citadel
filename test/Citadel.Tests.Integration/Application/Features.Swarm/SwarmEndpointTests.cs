using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class SwarmEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<ISwarmConnector> connector = new(MockBehavior.Strict);
    private Guid platformId;
    private Guid taskContainerId;
    private Guid otherPlatformId;
    private Guid standalonePlatformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
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
                    ["primary-network"],
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

    [Theory]
    [InlineData("nodes", "primary-node")]
    [InlineData("services", "primary-service")]
    [InlineData("tasks", "primary-task-1")]
    [InlineData("networks", "primary-network")]
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
            taskCount, taskCount, "Completed", null, ["80:80/tcp"], [$"{prefix}-network"],
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

    private sealed class FakeConnectorFactory(ISwarmConnector value) : IConnectorFactory<ISwarmConnector>
    {
        public ISwarmConnector GetConnector(PlatformConnectorType type) => value;
    }
}
