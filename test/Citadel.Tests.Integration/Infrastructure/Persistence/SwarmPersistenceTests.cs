using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class SwarmPersistenceTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
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
