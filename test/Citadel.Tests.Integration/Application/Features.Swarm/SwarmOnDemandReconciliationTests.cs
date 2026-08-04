using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Swarm;

public sealed class SwarmOnDemandReconciliationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<ISwarmConnector> connector = new();
    private Guid platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        var observedAt = new DateTimeOffset(2026, 8, 4, 12, 0, 0, TimeSpan.Zero);
        connector
            .Setup(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNodeResult>>(
            [
                new SwarmNodeResult(
                    "live-node", 1, "live-manager", "Manager", true, "Reachable", "Ready", null,
                    "Active", "28.0", "linux", "x86_64", "10.0.0.1", new Dictionary<string, string>(),
                    1, 1, observedAt, observedAt)
            ]));
        connector
            .Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmServiceResult>>(
            [
                new SwarmServiceResult(
                    "live-service", 1, "live-web", "Replicated", "nginx:latest", 1, 1,
                    "Completed", null, ["80:80/tcp"], ["live-network"], ["live-secret"], ["live-config"],
                    new Dictionary<string, string>(), observedAt, observedAt)
            ]));
        connector
            .Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>(
            [
                new SwarmTaskResult(
                    "live-task", 1, "live-web.1", "live-service", 1, "live-node", "Running", "Running",
                    null, null, "nginx:latest", ["80/tcp"], observedAt, observedAt, observedAt)
            ]));
        connector
            .Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>(
            [
                new SwarmNetworkResult(
                    "live-network", "live-overlay", "Swarm", "overlay", true, false, false, true, false,
                    ["10.0.0.0/24"], new Dictionary<string, string>(), observedAt)
            ]));
        connector
            .Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>(
            [
                new SwarmSecretResult(
                    "live-secret", 1, "live-secret-name", null, new Dictionary<string, string>(),
                    observedAt, observedAt)
            ]));
        connector
            .Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>(
            [
                new SwarmConfigResult(
                    "live-config", 1, "live-config-name", null, new Dictionary<string, string>(),
                    observedAt, observedAt)
            ]));

        services.ReplaceService<IConnectorFactory<ISwarmConnector>>(new FakeConnectorFactory(connector.Object));
        services.AddHostedService<DbWriteWorker>();
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = CreateSwarmPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
    }

    [Fact]
    public async Task EmptyInventoryEndpoint_ShouldReconcilePersistAndReturnSnapshot()
    {
        var response = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/swarm/nodes",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        Assert.Equal("live-manager", Assert.Single(await uow.Swarm.GetNodesAsync(platformId, TestContext.Current.CancellationToken)).Hostname);
        Assert.Equal("live-web", Assert.Single(await uow.Swarm.GetServicesAsync(platformId, TestContext.Current.CancellationToken)).Name);
        Assert.Equal("live-task", Assert.Single(await uow.Swarm.GetTasksAsync(platformId, 10, TestContext.Current.CancellationToken)).DockerTaskId);
        Assert.Equal("live-overlay", Assert.Single(await uow.Swarm.GetNetworksAsync(platformId, TestContext.Current.CancellationToken)).Name);
        Assert.Equal("live-secret-name", Assert.Single(await uow.Swarm.GetSecretsAsync(platformId, TestContext.Current.CancellationToken)).Name);
        Assert.Equal("live-config-name", Assert.Single(await uow.Swarm.GetConfigsAsync(platformId, TestContext.Current.CancellationToken)).Name);

        connector.Verify(value => value.ListNodesAsync(It.IsAny<ListSwarmNodesCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        connector.Verify(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()), Times.Once);
    }

    private static Platform CreateSwarmPlatform() => new(
        "swarm-on-demand",
        "https://swarm-on-demand.test",
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
            NodeID: "manager-node",
            NodeAddr: "10.0.0.1",
            LocalNodeState: "Active",
            ControlAvailable: true,
            Nodes: 1,
            Managers: 1,
            DaemonId: "swarm-on-demand-daemon",
            ContainerCount: 0,
            ContainersRunning: 0,
            ContainersPaused: 0,
            ContainersStopped: 0,
            ClusterId: "swarm-on-demand-cluster"),
        clusterId: "swarm-on-demand-cluster");

    private sealed class FakeConnectorFactory(ISwarmConnector value) : IConnectorFactory<ISwarmConnector>
    {
        public ISwarmConnector GetConnector(PlatformConnectorType type) => value;
    }
}
