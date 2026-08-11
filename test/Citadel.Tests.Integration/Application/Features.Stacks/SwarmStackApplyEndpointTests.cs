using System.Collections.Immutable;
using System.Net;
using System.Runtime.CompilerServices;
using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Stacks;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public sealed class SwarmStackApplyEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IStackConnector> stackConnector = new();
    private readonly Mock<ISwarmConnector> swarmConnector = new();
    private readonly Mock<IConnectorFactory<IStackConnector>> stackConnectorFactory = new();
    private readonly Mock<IConnectorFactory<ISwarmConnector>> swarmConnectorFactory = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerConnectorFactory = new();
    private readonly Mock<IPlatformConnector> platformConnector = new();
    private readonly Mock<IConnectorFactory<IPlatformConnector>> platformConnectorFactory = new();
    private Guid stackId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDbWorkQueue>();
        services.RemoveAll<IConnectorFactory<IStackConnector>>();
        services.RemoveAll<IConnectorFactory<ISwarmConnector>>();
        services.RemoveAll<IConnectorFactory<IContainerConnector>>();
        services.RemoveAll<IConnectorFactory<IPlatformConnector>>();
        services.RemoveAll<IResourceBindingResolver>();

        services
            .AddHostedService<DbWriteWorker>()
            .AddSingleton<IDbWorkQueue, DbWorkQueue>()
            .AddSingleton(stackConnectorFactory.Object)
            .AddSingleton(swarmConnectorFactory.Object)
            .AddSingleton(containerConnectorFactory.Object)
            .AddSingleton(platformConnectorFactory.Object)
            .AddSingleton<IResourceBindingResolver>(new EmptyResourceBindingResolver());

        stackConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(stackConnector.Object);
        swarmConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(swarmConnector.Object);
        platformConnectorFactory
            .Setup(factory => factory.GetConnector(PlatformConnectorType.Agent))
            .Returns(platformConnector.Object);
        platformConnector
            .Setup(connector => connector.GetPlatformAsync(
                It.IsAny<GetPlatformCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new PlatformResult(
                "swarm-manager",
                "https://swarm.example.test",
                0,
                0,
                0,
                4,
                1024,
                "28.0.0",
                "1.0.0",
                CreateManagerDescriptor(),
                ClusterId: "cluster-1")));
        stackConnector
            .Setup(connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulApplyStream());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = new Platform(
            "swarm-manager",
            "https://swarm.example.test",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 4,
            memTotal: 1024,
            serverVersion: "28.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: CreateManagerDescriptor(),
            clusterId: "cluster-1");
        var stack = Stack.Create(
            "endpoint-stack",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack(
                "services:\n  api:\n    image: nginx:latest\n",
                StackUpdateBehavior.Disabled,
                DestroyBeforeDeploy: false,
                ProjectName: "endpoint-stack"),
            driftPolicy: StackDriftPolicy.Disabled,
            platform: platform);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        stackId = stack.Id;

        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                ImmutableDictionary<string, Guid>.Empty));

        var labels = new Dictionary<string, string>
        {
            ["com.docker.stack.namespace"] = "endpoint-stack",
            [CitadelLabels.Managed] = "true",
            [CitadelLabels.StackId] = stack.Id.ToString("D"),
            [CitadelLabels.ReleaseId] = stack.CurrentStackReleaseId.ToString("D")
        };
        var serviceReads = 0;
        swarmConnector
            .Setup(connector => connector.ListServicesAsync(
                It.IsAny<ListSwarmServicesCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Interlocked.Increment(ref serviceReads) == 1
                ? Result.Success<IReadOnlyList<SwarmServiceResult>>([])
                : Result.Success<IReadOnlyList<SwarmServiceResult>>(
                [
                    new SwarmServiceResult(
                        "service-id", 1, "endpoint-stack_api", "replicated", "nginx:latest", 1, 1,
                        "Completed", null, [], [], [], [], labels, null, null)
                ]));
        swarmConnector
            .Setup(connector => connector.ListTasksAsync(
                It.IsAny<ListSwarmTasksCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>([]));
        swarmConnector
            .Setup(connector => connector.ListNetworksAsync(
                It.IsAny<ListSwarmNetworksCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>([]));
        swarmConnector
            .Setup(connector => connector.ListSecretsAsync(
                It.IsAny<ListSwarmSecretsCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>([]));
        swarmConnector
            .Setup(connector => connector.ListConfigsAsync(
                It.IsAny<ListSwarmConfigsCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>([]));
    }

    [Fact]
    public async Task ApplyEndpoint_ShouldDispatchSwarmModeAndPersistHealthyRelease()
    {
        using var response = await Client.PostAsync(
            "/api/v1/stacks/apply",
            new StringContent($$"""{"id":"{{stackId}}","recreate":false}""", Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Swarm Stack converged successfully", responseBody, StringComparison.Ordinal);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.Is<StackApplyCommand>(command => command.OrchestrationMode == StackOrchestrationMode.DockerSwarm),
                It.IsAny<CancellationToken>()),
            Times.Once);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(StackReleaseStatus.Healthy, persisted?.CurrentStackRelease?.Status);
    }

    [Fact]
    public async Task ApplyEndpoint_ShouldRejectUndefinedRequiredComposeVariableBeforeDockerMutation()
    {
        await using (var setupScope = Services.CreateAsyncScope())
        {
            var uow = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
            Assert.NotNull(stack?.CurrentStackRelease);
            stack.CurrentStackRelease.UpdateSpec(new ManualStack(
                "services:\n  api:\n    image: nginx:latest\n    environment:\n      TOKEN: ${MISSING_TOKEN}\n",
                StackUpdateBehavior.Disabled,
                DestroyBeforeDeploy: false,
                ProjectName: "endpoint-stack"));
            await uow.Stacks.UpdateAsync(stack, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.PostAsync(
            "/api/v1/stacks/apply",
            new StringContent($$"""{"id":"{{stackId}}","recreate":false}""", Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Required Compose variable 'MISSING_TOKEN'", responseBody, StringComparison.Ordinal);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.IsAny<StackApplyCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await verificationUow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
        Assert.Equal(ResourceControlState.Idle, persisted?.ControlState);
        Assert.Equal(StackReleaseStatus.Failed, persisted?.CurrentStackRelease?.Status);
    }

    [Fact]
    public async Task ApplyEndpoint_ShouldAllowComposeDefaultWithoutBinding()
    {
        await using (var setupScope = Services.CreateAsyncScope())
        {
            var uow = setupScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = await uow.Stacks.GetAsync(stackId, TestContext.Current.CancellationToken);
            Assert.NotNull(stack?.CurrentStackRelease);
            stack.CurrentStackRelease.UpdateSpec(new ManualStack(
                "services:\n  api:\n    image: nginx:latest\n    environment:\n      TOKEN: ${OPTIONAL_TOKEN:-fallback}\n",
                StackUpdateBehavior.Disabled,
                DestroyBeforeDeploy: false,
                ProjectName: "endpoint-stack"));
            await uow.Stacks.UpdateAsync(stack, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.PostAsync(
            "/api/v1/stacks/apply",
            new StringContent($$"""{"id":"{{stackId}}","recreate":false}""", Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Swarm Stack converged successfully", responseBody, StringComparison.Ordinal);
        stackConnector.Verify(
            connector => connector.StackApplyAsync(
                It.Is<StackApplyCommand>(command => command.OrchestrationMode == StackOrchestrationMode.DockerSwarm),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static async IAsyncEnumerable<StackApplyResult> SuccessfulApplyStream(
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        yield return StackApplyResult.Finished(0);
        await Task.CompletedTask;
    }

    private static DockerSwarmPlatformDescriptor CreateManagerDescriptor() => new(
        NodeID: "node-1",
        NodeAddr: "10.0.0.1",
        LocalNodeState: "Active",
        ControlAvailable: true,
        Nodes: 1,
        Managers: 1,
        DaemonId: "daemon-1",
        ContainerCount: 0,
        ContainersRunning: 0,
        ContainersPaused: 0,
        ContainersStopped: 0,
        ClusterId: "cluster-1");

    private sealed class EmptyResourceBindingResolver : IResourceBindingResolver
    {
        public Task<Result<ResolvedResourceBindings>> ResolveAsync(
            ResourceBindingScope scope,
            Guid resourceId,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(new ResolvedResourceBindings([], [], [], 0, 0)));
    }
}
