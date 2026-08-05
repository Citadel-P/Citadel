using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Npgsql;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class SwarmPersistenceTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ProjectionRepository_ShouldBulkUpsertAndRemoveAllInventoryKinds()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = CreateSwarmPlatform("swarm-projections", "cluster-projections");
        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        var observedAt = DateTimeOffset.UtcNow;
        var node = new SwarmNodeProjection(
            platform.Id, "node-1", 1, "manager-1", "Manager", true, "Reachable", "Ready",
            null, "Active", "28.0", "linux", "x86_64", "10.0.0.1",
            new Dictionary<string, string> { ["zone"] = "west" }, 2, 3,
            observedAt.AddHours(-1), observedAt, observedAt, false);
        var service = new SwarmServiceProjection(
            platform.Id, "service-1", 2, "web", "Replicated", "nginx:latest", 1, 2,
            "Completed", null, ["80:80/tcp"], ["network-1"], ["secret-1"], ["config-1"],
            new Dictionary<string, string> { ["team"] = "ops" }, observedAt, observedAt,
            observedAt, false, SwarmServiceOwnership.DockerStackExternal,
            DockerStackNamespace: "sample-stack",
            OwnershipDiagnostic: "Orphaned Citadel metadata");
        var task = new SwarmTaskProjection(
            platform.Id, "task-1", 3, "web.1", "service-1", "web", 1, "node-1",
            "manager-1", "Running", "Running", null, null, "nginx:latest", ["80/tcp"],
            observedAt, observedAt, observedAt, observedAt, false);
        var network = new SwarmNetworkProjection(
            platform.Id, "network-1", "frontend", "Swarm", "overlay", true, false, false,
            true, false, ["10.0.0.0/24"], ["web"],
            new Dictionary<string, string> { ["tier"] = "public" }, observedAt, observedAt, false);
        var secret = new SwarmSecretProjection(
            platform.Id, "secret-1", 4, "password", null, ["web"],
            new Dictionary<string, string> { ["owner"] = "ops" }, observedAt, observedAt,
            observedAt, false);
        var config = new SwarmConfigProjection(
            platform.Id, "config-1", 5, "nginx-conf", null, ["web"],
            new Dictionary<string, string> { ["owner"] = "ops" }, observedAt, observedAt,
            observedAt, false);

        await uow.Swarm.ReplaceAsync(
            platform.Id,
            new SwarmProjectionSnapshot([node], [service], [task], [network], [secret], [config]),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var storedNode = Assert.Single(await uow.Swarm.GetNodesAsync(platform.Id, cancellationToken));
        var storedService = Assert.Single(await uow.Swarm.GetServicesAsync(platform.Id, cancellationToken));
        var storedTask = Assert.Single(await uow.Swarm.GetTasksAsync(platform.Id, 10, cancellationToken));
        var storedNetwork = Assert.Single(await uow.Swarm.GetNetworksAsync(platform.Id, cancellationToken));
        var storedSecret = Assert.Single(await uow.Swarm.GetSecretsAsync(platform.Id, cancellationToken));
        var storedConfig = Assert.Single(await uow.Swarm.GetConfigsAsync(platform.Id, cancellationToken));
        Assert.Equal("west", storedNode.Labels["zone"]);
        Assert.Equal(["80:80/tcp"], storedService.Ports);
        Assert.Equal(SwarmServiceOwnership.DockerStackExternal, storedService.Ownership);
        Assert.Equal("sample-stack", storedService.DockerStackNamespace);
        Assert.Equal("Orphaned Citadel metadata", storedService.OwnershipDiagnostic);
        await using (var connection = new NpgsqlConnection(ConnectionString))
        {
            await connection.OpenAsync(cancellationToken);
            await using var command = connection.CreateCommand();
            command.CommandText = "SELECT Ownership FROM SwarmServiceProjections WHERE PlatformId = @platformId";
            command.Parameters.AddWithValue("platformId", platform.Id);
            Assert.Equal(
                nameof(SwarmServiceOwnership.DockerStackExternal),
                await command.ExecuteScalarAsync(cancellationToken));
        }
        Assert.Equal("manager-1", storedTask.NodeHostname);
        Assert.Equal(["10.0.0.0/24"], storedNetwork.Subnets);
        Assert.Equal(["web"], storedSecret.ServiceNames);
        Assert.Equal("ops", storedConfig.Labels["owner"]);

        await uow.Swarm.ReplaceAsync(
            platform.Id,
            new SwarmProjectionSnapshot([node with { Hostname = "manager-updated" }], [], [], [], [], []),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        Assert.Equal("manager-updated", Assert.Single(await uow.Swarm.GetNodesAsync(platform.Id, cancellationToken)).Hostname);
        Assert.Empty(await uow.Swarm.GetServicesAsync(platform.Id, cancellationToken));
        Assert.Empty(await uow.Swarm.GetTasksAsync(platform.Id, 10, cancellationToken));
        Assert.Empty(await uow.Swarm.GetNetworksAsync(platform.Id, cancellationToken));
        Assert.Empty(await uow.Swarm.GetSecretsAsync(platform.Id, cancellationToken));
        Assert.Empty(await uow.Swarm.GetConfigsAsync(platform.Id, cancellationToken));

        await uow.Swarm.MarkStaleAsync(platform.Id, cancellationToken);
        await uow.CommitAsync(cancellationToken);
        Assert.True(Assert.Single(await uow.Swarm.GetNodesAsync(platform.Id, cancellationToken)).IsStale);
        var summary = await uow.Swarm.GetSummaryAsync(platform.Id, cancellationToken);
        Assert.True(summary.IsStale);
        Assert.Equal(1, summary.NodeCount);
        Assert.Equal(1, summary.ManagerCount);
        Assert.Equal(0, summary.ServiceCount);
        Assert.Equal(0, summary.RunningTaskCount);
        Assert.Equal(0, summary.DesiredTaskCount);
        Assert.Equal(0, summary.NetworkCount);
    }

    [Fact]
    public async Task ProjectionRepository_WhenALaterWriteFails_ShouldRollbackEarlierReplacements()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var platform = CreateSwarmPlatform("swarm-rollback", "cluster-rollback");
        var observedAt = DateTimeOffset.UtcNow;
        var originalNode = new SwarmNodeProjection(
            platform.Id, "node-1", 1, "original", "Manager", true, "Reachable", "Ready",
            null, "Active", "28.0", "linux", "x86_64", "10.0.0.1", new Dictionary<string, string>(), 1, 1,
            null, null, observedAt, false);
        var originalService = new SwarmServiceProjection(
            platform.Id, "service-1", 1, "web", "Replicated", "nginx:latest", 1, 1,
            string.Empty, null, [], [], [], [], new Dictionary<string, string>(), null, null, observedAt, false);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);
            await uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot([originalNode], [originalService], [], [], [], []),
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var invalidConfig = new SwarmConfigProjection(
                platform.Id, "config-1", 1, null!, null, [], new Dictionary<string, string>(), null, null, observedAt, false);
            await Assert.ThrowsAnyAsync<Exception>(() => uow.Swarm.ReplaceAsync(
                platform.Id,
                new SwarmProjectionSnapshot(
                    [originalNode with { Hostname = "partially-updated" }],
                    [], [], [], [], [invalidConfig]),
                cancellationToken));
        }

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.Equal(
            "original",
            Assert.Single(await verificationUow.Swarm.GetNodesAsync(platform.Id, cancellationToken)).Hostname);
        Assert.Equal(
            "web",
            Assert.Single(await verificationUow.Swarm.GetServicesAsync(platform.Id, cancellationToken)).Name);
    }

    [Fact]
    public async Task Repositories_ShouldInferWorkloadTypeFromPlatformRelationship()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = CreateSwarmPlatform("swarm-roundtrip", "cluster-roundtrip");
        var deployment = new Deployment(
            "swarm-deployment",
            Constants.SystemId,
            platform.Id,
            new DeploymentSpec(
                new ExternalImage(Constants.DefaultRegistryId, "nginx:latest"),
                UpdateBehavior.Disabled),
            platform: platform);
        var stack = Stack.Create(
            "swarm-stack",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services: {}", StackUpdateBehavior.Disabled),
            platform: platform);

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Deployments.AddAsync(deployment, cancellationToken);
        await uow.Stacks.AddAsync(stack, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var storedPlatform = await uow.Platforms.GetByIdAsync(platform.Id, cancellationToken);
        var storedDeployment = await uow.Deployments.GetAsync(deployment.Id, cancellationToken);
        var storedStack = await uow.Stacks.GetAsync(stack.Id, cancellationToken);

        Assert.Equal("cluster-roundtrip", storedPlatform?.ClusterId);
        Assert.Equal(PlatformType.DockerSwarm, storedPlatform?.PlatformDescriptor.Type);
        Assert.Equal(PlatformType.DockerSwarm, storedDeployment?.Platform?.PlatformDescriptor.Type);
        Assert.Equal(PlatformType.DockerSwarm, storedStack?.CurrentStackRelease?.Platform?.PlatformDescriptor.Type);
    }

    [Fact]
    public async Task PlatformRepository_ShouldIgnoreDuplicateSwarmClusterIdentity()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Platforms.AddAsync(
            CreateSwarmPlatform("swarm-a", "shared-cluster"),
            cancellationToken);

        var result = await uow.Platforms.AddAsync(
            CreateSwarmPlatform("swarm-b", "shared-cluster"),
            cancellationToken);

        Assert.Equal(0, result);
        Assert.Null(await uow.Platforms.GetByNameAsync("swarm-b", cancellationToken));
    }

    private static Platform CreateSwarmPlatform(string name, string clusterId) => new(
        name,
        $"https://{name}.test",
        networkCount: 0,
        volumeCount: 0,
        imageCount: 0,
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
            Nodes: 3,
            Managers: 1,
            DaemonId: $"daemon-{name}",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0,
            ClusterId: clusterId),
        clusterId: clusterId);
}
