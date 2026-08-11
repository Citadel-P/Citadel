using System.Text.Json;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;

namespace Tests.Integration.Application.Features.Platforms;

public sealed class SwarmNodeAgentLifecycleTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<ISwarmReconciliationCoordinator> reconciliationCoordinator = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        reconciliationCoordinator
            .Setup(value => value.RefreshAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        services.RemoveAll<ISwarmReconciliationCoordinator>();
        services.AddSingleton(reconciliationCoordinator.Object);
        var managerIdentityValidator = new Mock<ISwarmManagerIdentityValidator>();
        managerIdentityValidator
            .Setup(value => value.ValidateAsync(It.IsAny<Platform>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        services.RemoveAll<ISwarmManagerIdentityValidator>();
        services.AddSingleton(managerIdentityValidator.Object);
    }

    [Fact]
    public async Task Coverage_WithOnlyPinnedManager_ShouldReportCompleteWithoutInstallation()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("Complete", body.RootElement.GetProperty("state").GetString());
        Assert.False(body.RootElement.GetProperty("isInstalled").GetBoolean());
        Assert.Equal(1, body.RootElement.GetProperty("coveredNodes").GetInt32());
        Assert.Equal(1, body.RootElement.GetProperty("eligibleNodes").GetInt32());
        Assert.Equal(1, body.RootElement.GetProperty("totalNodes").GetInt32());
        Assert.True(body.RootElement.GetProperty("canManageNodeAgents").GetBoolean());
    }

    [Theory]
    [InlineData("POST", "install")]
    [InlineData("POST", "repair")]
    [InlineData("POST", "upgrade")]
    [InlineData("DELETE", "")]
    public async Task LifecycleEndpoints_ShouldRequirePlatformScopedManageNodeAgentsPermission(
        string method,
        string action)
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var operatorSubject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, platform.Id, PermissionLevel.Execute)
            ]);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(operatorSubject.UserId, operatorSubject.ActorId));
        var suffix = string.IsNullOrEmpty(action) ? string.Empty : $"/{action}";

        using var response = await Client.SendAsync(
            new HttpRequestMessage(
                new HttpMethod(method),
                $"/api/v1/platforms/{platform.Id:D}/node-agents{suffix}"),
            TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
    }

    [Fact]
    public async Task Coverage_ShouldTreatDockerX8664NodeAsSupportedAmd64Satellite()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTimeOffset.UtcNow;
        var worker = new SwarmNodeProjection(
            platform.Id,
            "worker-node",
            1,
            "worker",
            "Worker",
            false,
            string.Empty,
            "Ready",
            null,
            "Active",
            "29.0",
            "linux",
            "x86_64",
            "10.0.0.2",
            new Dictionary<string, string>(),
            0,
            0,
            now,
            now,
            now,
            false);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var manager = Assert.Single(await uow.Swarm.GetNodesAsync(
                platform.Id,
                TestContext.Current.CancellationToken));
            await uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot([manager, worker], [], [], [], [], []),
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("NotInstalled", body.RootElement.GetProperty("state").GetString());
        Assert.Equal(2, body.RootElement.GetProperty("eligibleNodes").GetInt32());
        Assert.Equal(0, body.RootElement.GetProperty("unsupportedNodes").GetInt32());
    }

    [Fact]
    public async Task Coverage_ShouldKeepActiveDownWorkerInEligibleTotal()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTimeOffset.UtcNow;
        var worker = new SwarmNodeProjection(
            platform.Id,
            "worker-node",
            1,
            "worker",
            "Worker",
            false,
            string.Empty,
            "Down",
            "worker unavailable",
            "Active",
            "29.0",
            "linux",
            "amd64",
            "10.0.0.2",
            new Dictionary<string, string>(),
            0,
            0,
            now,
            now,
            now,
            false);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var manager = Assert.Single(await uow.Swarm.GetNodesAsync(
                platform.Id,
                TestContext.Current.CancellationToken));
            await uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot([manager, worker], [], [], [], [], []),
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("NotInstalled", body.RootElement.GetProperty("state").GetString());
        Assert.Equal(2, body.RootElement.GetProperty("eligibleNodes").GetInt32());
        Assert.Equal(1, body.RootElement.GetProperty("coveredNodes").GetInt32());
        var workerCoverage = Assert.Single(
            body.RootElement.GetProperty("nodes").EnumerateArray(),
            node => node.GetProperty("dockerNodeId").GetString() == "worker-node");
        Assert.True(workerCoverage.GetProperty("eligible").GetBoolean());
        Assert.False(workerCoverage.GetProperty("schedulable").GetBoolean());
        Assert.Equal("Missing", workerCoverage.GetProperty("agentConnectionState").GetString());
    }

    [Fact]
    public async Task Coverage_ShouldReportOwnedServiceDrift_WhenInstalledServiceIsMissing()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTimeOffset.UtcNow;
        var installation = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            "missing-service",
            $"citadel-node-agent-{platform.Id:N}",
            "ghcr.io/citadel-p/citadel.agent:latest",
            "sha256:test",
            null,
            null,
            SwarmNodeAgentDesiredState.Installed,
            Guid.CreateVersion7(),
            SwarmNodeAgentOperationKind.Install,
            SwarmNodeAgentOperationState.Completed,
            now.UtcDateTime,
            Constants.SystemId,
            null,
            now.UtcDateTime,
            now.UtcDateTime);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var manager = Assert.Single(await uow.Swarm.GetNodesAsync(
                platform.Id,
                TestContext.Current.CancellationToken));
            await uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot([manager], [], [], [], [], []),
                TestContext.Current.CancellationToken);
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(
                installation,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("Partial", body.RootElement.GetProperty("state").GetString());
        Assert.True(body.RootElement.GetProperty("isInstalled").GetBoolean());
        Assert.Contains(
            body.RootElement.GetProperty("reasons").EnumerateArray(),
            item => item.GetString() == "NodeAgentServiceDrifted");
    }

    [Fact]
    public async Task Coverage_WithInstalledManagerOnlyDataPlane_ShouldRemainCompleteWithoutSatelliteService()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTime.UtcNow;
        var installation = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            null,
            $"citadel-node-agent-{platform.Id:N}",
            "ghcr.io/citadel-p/citadel.agent:latest",
            "sha256:test",
            null,
            null,
            SwarmNodeAgentDesiredState.Installed,
            Guid.CreateVersion7(),
            SwarmNodeAgentOperationKind.Install,
            SwarmNodeAgentOperationState.Completed,
            now,
            Constants.SystemId,
            null,
            now,
            now);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(
                installation,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal("Complete", body.RootElement.GetProperty("state").GetString());
        Assert.True(body.RootElement.GetProperty("isInstalled").GetBoolean());
        Assert.DoesNotContain(
            body.RootElement.GetProperty("reasons").EnumerateArray(),
            item => item.GetString() == "NodeAgentServiceDrifted");
    }

    [Theory]
    [InlineData(SwarmNodeAgentOperationKind.Install, SwarmNodeAgentDesiredState.Installed, "Installing")]
    [InlineData(SwarmNodeAgentOperationKind.Remove, SwarmNodeAgentDesiredState.Removed, "Removing")]
    public async Task Coverage_ShouldExposeRunningLifecycleState(
        SwarmNodeAgentOperationKind operationKind,
        SwarmNodeAgentDesiredState desiredState,
        string expectedState)
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTime.UtcNow;
        var installation = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            null,
            $"citadel-node-agent-{platform.Id:N}",
            "ghcr.io/citadel-p/citadel.agent:latest",
            "sha256:test",
            null,
            null,
            desiredState,
            Guid.CreateVersion7(),
            operationKind,
            SwarmNodeAgentOperationState.Running,
            now,
            Constants.SystemId,
            null,
            now,
            now);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(
                installation,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/platforms/{platform.Id:D}/node-agent-coverage",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var body = JsonDocument.Parse(
            await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        Assert.Equal(expectedState, body.RootElement.GetProperty("state").GetString());
    }

    [Fact]
    public async Task Remove_ShouldRevokeNodeBindingsAndPersistRemovedDesiredState()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTime.UtcNow;
        var installation = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            null,
            $"citadel-node-agent-{platform.Id:N}",
            "ghcr.io/citadel/citadel-agent:latest",
            "sha256:test",
            null,
            null,
            SwarmNodeAgentDesiredState.Installed,
            null,
            null,
            null,
            null,
            null,
            null,
            now,
            now);
        var binding = new EdgeAgentBinding(
            Guid.CreateVersion7(),
            platform.Id,
            EdgeAgentResourceType.Platform,
            platform.Id,
            Guid.CreateVersion7(),
            "public-key",
            $"SHA256:{Guid.NewGuid():N}",
            EdgeAgentConnectionStatus.Offline,
            null,
            now,
            now,
            "test",
            "worker",
            "{}",
            Constants.EdgeAgentProtocolVersion,
            null,
            now,
            now,
            EdgeAgentProfile.SwarmNode,
            platform.ClusterId,
            "worker-node",
            "worker-daemon");

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(
                installation,
                TestContext.Current.CancellationToken);
            await uow.EdgeAgents.AddBindingAsync(binding, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var executeScope = Services.CreateAsyncScope())
        {
            var lifecycle = executeScope.ServiceProvider.GetRequiredService<ISwarmNodeAgentLifecycleService>();
            var result = await lifecycle.ExecuteAsync(
                platform.Id,
                SwarmNodeAgentOperationKind.Remove,
                string.Empty,
                Constants.SystemId,
                static _ => { },
                TestContext.Current.CancellationToken);
            Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var removed = await assertUow.EdgeAgents.GetNodeAgentInstallationAsync(
            platform.Id,
            TestContext.Current.CancellationToken);
        var revoked = Assert.Single(await assertUow.EdgeAgents.GetNodeBindingsAsync(
            platform.Id,
            TestContext.Current.CancellationToken));
        Assert.Equal(SwarmNodeAgentDesiredState.Removed, removed?.DesiredState);
        Assert.Equal(SwarmNodeAgentOperationState.Completed, removed?.OperationState);
        Assert.True(revoked?.IsRevoked);
        var activities = await assertUow.ActivityEventRepository.GetPagedAsync(
            platform.Id,
            ActivityResourceType.Platform,
            ActivityEventType.PlatformNodeAgentLifecycle,
            1,
            10,
            TestContext.Current.CancellationToken);
        var lifecycleActivities = activities.Items.ToArray();
        Assert.Equal(2, lifecycleActivities.Length);
        Assert.Contains(
            lifecycleActivities,
            activity => activity.Status == ActivityStatus.Information
                        && activity.Info is PlatformNodeAgentLifecycle
                        {
                            State: SwarmNodeAgentOperationState.Running
                        });
        Assert.Contains(
            lifecycleActivities,
            activity => activity.Status == ActivityStatus.Success
                        && activity.Info is PlatformNodeAgentLifecycle
                        {
                            State: SwarmNodeAgentOperationState.Completed
                        });
    }

    [Fact]
    public async Task Install_AfterRemoval_ShouldRestoreInstalledDesiredState()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTime.UtcNow;
        var removed = new SwarmNodeAgentInstallation(
            platform.Id,
            platform.ClusterId!,
            "manager-node",
            "manager-daemon",
            null,
            $"citadel-node-agent-{platform.Id:N}",
            "ghcr.io/citadel-p/citadel.agent:latest",
            string.Empty,
            null,
            null,
            SwarmNodeAgentDesiredState.Removed,
            null,
            null,
            null,
            null,
            null,
            null,
            now,
            now);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(removed, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var executeScope = Services.CreateAsyncScope())
        {
            var lifecycle = executeScope.ServiceProvider.GetRequiredService<ISwarmNodeAgentLifecycleService>();
            var result = await lifecycle.ExecuteAsync(
                platform.Id,
                SwarmNodeAgentOperationKind.Install,
                "https://core.example.test",
                Constants.SystemId,
                static _ => { },
                TestContext.Current.CancellationToken);
            Assert.True(result.IsSuccess(out _, out var error), error?.Message);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var installation = await assertUow.EdgeAgents.GetNodeAgentInstallationAsync(
            platform.Id,
            TestContext.Current.CancellationToken);
        Assert.Equal(SwarmNodeAgentDesiredState.Installed, installation?.DesiredState);
        Assert.Equal(SwarmNodeAgentOperationState.Completed, installation?.OperationState);
    }

    [Fact]
    public async Task NodeRuntimeProjectionState_ShouldRoundTripAndUpdateWithoutDynamicDapperParameters()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var observedAt = DateTimeOffset.UtcNow;
        var initial = new SwarmNodeRuntimeProjectionState(
            platform.Id,
            "worker-node",
            4,
            observedAt.AddSeconds(-2),
            observedAt.AddSeconds(-1),
            observedAt.AddSeconds(-1),
            false,
            null,
            null,
            observedAt.AddMinutes(-1),
            null,
            observedAt,
            "1.2.3",
            "29.0");

        await using (var writeScope = Services.CreateAsyncScope())
        {
            var uow = writeScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Equal(1, await uow.Swarm.UpsertNodeRuntimeStateAsync(initial, TestContext.Current.CancellationToken));
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var staleAt = observedAt.AddMinutes(1);
        await using (var updateScope = Services.CreateAsyncScope())
        {
            var uow = updateScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Equal(1, await uow.Swarm.UpsertNodeRuntimeStateAsync(initial with
            {
                ReconciliationGeneration = 5,
                IsStale = true,
                StaleSince = staleAt,
                StaleReason = "Agent disconnected."
            }, TestContext.Current.CancellationToken));
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using var readScope = Services.CreateAsyncScope();
        var readUow = readScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await readUow.Swarm.GetNodeRuntimeStateAsync(
            platform.Id,
            "worker-node",
            TestContext.Current.CancellationToken);
        Assert.NotNull(persisted);
        Assert.Equal(5, persisted.ReconciliationGeneration);
        Assert.True(persisted.IsStale);
        Assert.Equal("Agent disconnected.", persisted.StaleReason);
        Assert.Equal(staleAt.ToUnixTimeMilliseconds(), persisted.StaleSince?.ToUnixTimeMilliseconds());
        Assert.Single(await readUow.Swarm.GetNodeRuntimeStatesAsync(
            platform.Id,
            TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Reconnect_ShouldRebindPersistedIdentity_WhenNodeRejoinsAfterRemovalGrace()
    {
        var platform = await CreatePlatformWithManagerNodeAsync();
        var now = DateTimeOffset.UtcNow;
        const string serviceId = "node-agent-service";
        const string taskId = "node-agent-task";
        const string newNodeId = "worker-node-new";
        var worker = new SwarmNodeProjection(
            platform.Id, newNodeId, 2, "worker", "Worker", false, string.Empty,
            "Ready", null, "Active", "29.0", "linux", "amd64", "10.0.0.2",
            new Dictionary<string, string>(), 1, 1, now, now, now, false);
        var service = new SwarmServiceProjection(
            platform.Id, serviceId, 3, $"citadel-node-agent-{platform.Id:N}", "Global",
            "ghcr.io/citadel-p/citadel.agent@sha256:test", 1, 1, "Completed", null,
            [], [], [], [], new Dictionary<string, string>
            {
                ["com.citadel.system"] = "true",
                ["com.citadel.system-role"] = "swarm-node-agent",
                ["com.citadel.platform-id"] = platform.Id.ToString("D"),
                ["com.citadel.swarm-cluster-id"] = platform.ClusterId!
            }, now, now, now, false, SwarmServiceOwnership.System);
        var task = new SwarmTaskProjection(
            platform.Id, taskId, 4, "node-agent.1", serviceId, service.Name, 1,
            newNodeId, worker.Hostname, "running", "running", null, null,
            service.Image, [], now, now, now, now, false);
        var oldHeartbeat = now.UtcDateTime.AddMinutes(-20);
        var agentId = Guid.CreateVersion7();
        var fingerprint = $"SHA256:{Guid.NewGuid():N}";
        var binding = new EdgeAgentBinding(
            Guid.CreateVersion7(), platform.Id, EdgeAgentResourceType.Platform, platform.Id,
            agentId, "public-key", fingerprint, EdgeAgentConnectionStatus.Offline,
            oldHeartbeat, now.UtcDateTime, oldHeartbeat, "test", "worker", "{}",
            Constants.EdgeAgentProtocolVersion, null, oldHeartbeat, oldHeartbeat,
            EdgeAgentProfile.SwarmNode, platform.ClusterId, "worker-node-old", "worker-daemon",
            "worker", "Worker", serviceId, "old-task", oldHeartbeat, oldHeartbeat);
        var installation = new SwarmNodeAgentInstallation(
            platform.Id, platform.ClusterId!, "manager-node", "manager-daemon", serviceId,
            service.Name, service.Image, "sha256:test", null, null,
            SwarmNodeAgentDesiredState.Installed, null, null, null, null, null, null,
            oldHeartbeat, oldHeartbeat);

        await using (var seedScope = Services.CreateAsyncScope())
        {
            var uow = seedScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var manager = Assert.Single(await uow.Swarm.GetNodesAsync(
                platform.Id,
                TestContext.Current.CancellationToken));
            await uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot([manager, worker], [service], [task], [], [], []),
                TestContext.Current.CancellationToken);
            await uow.EdgeAgents.UpsertNodeAgentInstallationAsync(installation, TestContext.Current.CancellationToken);
            await uow.EdgeAgents.AddBindingAsync(binding, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var recentReconnectScope = Services.CreateAsyncScope())
        {
            var management = recentReconnectScope.ServiceProvider.GetRequiredService<IEdgeAgentManagementService>();
            var result = await management.GetSwarmNodeReconnectBindingAsync(
                platform.Id,
                agentId,
                fingerprint,
                new EdgeAgentHeartbeatSnapshot(
                    true, "29.0", worker.Hostname, "test", "{}", "worker-daemon",
                    platform.ClusterId, newNodeId, worker.Hostname, serviceId, taskId, "Worker"),
                TestContext.Current.CancellationToken);
            Assert.True(result.IsFailure(out var error));
            Assert.Contains("grace", error?.Message, StringComparison.OrdinalIgnoreCase);
        }

        await using (var ageScope = Services.CreateAsyncScope())
        {
            var uow = ageScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.EdgeAgents.UpdateNodeBindingDisconnectedAsync(
                platform.Id,
                "worker-node-old",
                oldHeartbeat,
                TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var reconnectScope = Services.CreateAsyncScope())
        {
            var management = reconnectScope.ServiceProvider.GetRequiredService<IEdgeAgentManagementService>();
            var result = await management.GetSwarmNodeReconnectBindingAsync(
                platform.Id,
                agentId,
                fingerprint,
                new EdgeAgentHeartbeatSnapshot(
                    true, "29.0", worker.Hostname, "test", "{}", "worker-daemon",
                    platform.ClusterId, newNodeId, worker.Hostname, serviceId, taskId, "Worker"),
                TestContext.Current.CancellationToken);
            Assert.True(result.IsSuccess(out var rebound, out var error), error?.Message);
            Assert.Equal(newNodeId, rebound.DockerNodeId);
            Assert.Equal(taskId, rebound.LastObservedTaskId);
        }

        await using var assertScope = Services.CreateAsyncScope();
        var assertUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.Null(await assertUow.EdgeAgents.GetNodeBindingAsync(
            platform.Id, "worker-node-old", TestContext.Current.CancellationToken));
        var persisted = await assertUow.EdgeAgents.GetNodeBindingAsync(
            platform.Id, newNodeId, TestContext.Current.CancellationToken);
        Assert.NotNull(persisted);
        Assert.Equal(agentId, persisted.AgentId);
    }

    private async Task<Platform> CreatePlatformWithManagerNodeAsync()
    {
        var platform = new Platform(
            $"swarm-{Guid.CreateVersion7():N}",
            "https://swarm.example.test",
            0,
            0,
            0,
            4,
            1024,
            "29.0",
            "1.0",
            PlatformStatus.Online,
            PlatformConnectorType.Agent,
            new DockerSwarmPlatformDescriptor(
                "manager-node",
                "10.0.0.1",
                "Active",
                true,
                1,
                1,
                "manager-daemon",
                0,
                0,
                0,
                0,
                "cluster-test"),
            clusterId: "cluster-test");
        var now = DateTimeOffset.UtcNow;
        var manager = new SwarmNodeProjection(
            platform.Id,
            "manager-node",
            1,
            "manager",
            "Manager",
            true,
            "Reachable",
            "Ready",
            null,
            "Active",
            "29.0",
            "linux",
            "x86_64",
            "10.0.0.1",
            new Dictionary<string, string>(),
            0,
            0,
            now,
            now,
            now,
            false);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        await uow.Swarm.ReplaceAsync(
            platform.Id,
            new SwarmProjectionSnapshot([manager], [], [], [], [], []),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return platform;
    }
}
