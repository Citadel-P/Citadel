using Citadel.SharedModels.V1;
using Citadel.Volumes.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities;
using Domain.Entities.Platforms;
using Google.Protobuf;
using Infrastructure.Connectors.EdgeAgentConnectors;
using Moq;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class SwarmNodeRuntimeConnectorTests
{
    [Fact]
    public async Task VolumeMutations_WorkerNode_RouteStructuredCommandsToExactNode()
    {
        const string workerNodeId = "worker-node";
        var calls = new List<(string NodeId, EdgeAgentCommandKind Kind, byte[] Payload)>();
        var commandRouter = new Mock<IEdgeAgentCommandRouter>(MockBehavior.Strict);
        commandRouter
            .Setup(x => x.SendUnaryAsync(
                It.IsAny<Guid>(),
                workerNodeId,
                It.IsAny<EdgeAgentCommandKind>(),
                It.IsAny<byte[]>(),
                It.IsAny<TimeSpan>(),
                null,
                It.IsAny<CancellationToken>()))
            .Callback<Guid, string, EdgeAgentCommandKind, byte[], TimeSpan, string?, CancellationToken>(
                (_, nodeId, kind, payload, _, _, _) => calls.Add((nodeId, kind, payload)))
            .ReturnsAsync(EdgeAgentCommandRouterResult.Success(new VolumeResponse
            {
                Name = "restored-data",
                Driver = "local",
                Scope = "local"
            }.ToByteArray()));
        var platform = CreateSwarmPlatform("manager-node");
        var connector = new SwarmNodeRuntimeConnector(
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<IConnectorFactory<IVolumeConnector>>(),
            Mock.Of<IConnectorFactory<INetworkConnector>>(),
            commandRouter.Object);

        var created = await connector.CreateVolumeAsync(
            platform,
            workerNodeId,
            new CreateDockerVolumeCommand(
                platform.Address,
                "restored-data",
                "local",
                new Dictionary<string, string> { ["com.citadel.backup"] = "true" },
                new Dictionary<string, string>()),
            TestContext.Current.CancellationToken);
        var deleted = await connector.DeleteVolumeAsync(
            platform,
            workerNodeId,
            new DeleteDockerVolumeCommand(platform.Address, true, ["restored-data"]),
            TestContext.Current.CancellationToken);

        Assert.True(created.IsSuccess());
        Assert.True(deleted.IsSuccess());
        Assert.Collection(
            calls,
            call =>
            {
                Assert.Equal(workerNodeId, call.NodeId);
                Assert.Equal(EdgeAgentCommandKind.VolumeCreate, call.Kind);
                var request = CreateVolumeRequest.Parser.ParseFrom(call.Payload);
                Assert.Equal("restored-data", request.Name);
                Assert.Equal("local", request.Driver);
                Assert.Equal("true", request.Labels["com.citadel.backup"]);
            },
            call =>
            {
                Assert.Equal(workerNodeId, call.NodeId);
                Assert.Equal(EdgeAgentCommandKind.VolumeDelete, call.Kind);
                var request = RemoveVolumeRequest.Parser.ParseFrom(call.Payload);
                Assert.Equal(["restored-data"], request.Names);
                Assert.True(request.Force);
            });
    }

    [Fact]
    public async Task ExecBinaryAsync_WorkerNode_DisposeCancelsRoutedStream()
    {
        const string managerNodeId = "manager-node";
        const string workerNodeId = "worker-node";
        CancellationToken routedToken = default;
        var commandRouter = new Mock<IEdgeAgentCommandRouter>(MockBehavior.Strict);
        commandRouter
            .Setup(x => x.SendServerStreamAsync(
                It.IsAny<Guid>(),
                workerNodeId,
                EdgeAgentCommandKind.ContainerExecBinary,
                It.IsAny<byte[]>(),
                Timeout.InfiniteTimeSpan,
                null,
                It.IsAny<CancellationToken>()))
            .Returns((
                Guid _,
                string _,
                EdgeAgentCommandKind _,
                byte[] _,
                TimeSpan _,
                string? _,
                CancellationToken cancellationToken) =>
            {
                routedToken = cancellationToken;
                return EmptyCommandStream();
            });
        var platform = new Platform(
            "swarm",
            "http://localhost.docker",
            0,
            0,
            0,
            1,
            1,
            null,
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                managerNodeId,
                "192.168.65.3",
                "Active",
                true,
                1,
                1,
                "daemon-id",
                1,
                1,
                0,
                0));
        var connector = new SwarmNodeRuntimeConnector(
            Mock.Of<IConnectorFactory<IContainerConnector>>(),
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<IConnectorFactory<IVolumeConnector>>(),
            Mock.Of<IConnectorFactory<INetworkConnector>>(),
            commandRouter.Object);

        var result = await connector.ExecBinaryAsync(
            platform,
            workerNodeId,
            new ContainerBinaryExecRequest("helper", ["volume-helper", "list"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var execution, out _));
        Assert.True(routedToken.CanBeCanceled);
        await execution.DisposeAsync();
        Assert.True(routedToken.IsCancellationRequested);
    }

    [Fact]
    public async Task StreamContainerStatsAsync_ManagerNode_UsesDockerIdAndPlatformAddressInCorrectOrder()
    {
        const string managerNodeId = "manager-node";
        const string dockerContainerId = "6abd7abddd850295e921ce3a5e48ee0080bc015f56a525ffcdc840e6dc59442c";
        const string platformAddress = "http://localhost.docker";
        const int fetchIntervalMs = 2_000;
        StreamContainerStatsCommand? observedCommand = null;

        var containerConnector = new Mock<IContainerConnector>(MockBehavior.Strict);
        containerConnector
            .Setup(x => x.StreamContainerStatsAsync(
                It.IsAny<StreamContainerStatsCommand>(),
                It.IsAny<CancellationToken>()))
            .Returns((StreamContainerStatsCommand command, CancellationToken _) =>
            {
                observedCommand = command;
                return EmptyStatsStream();
            });
        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>(MockBehavior.Strict);
        connectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);
        var platform = new Platform(
            "swarm",
            platformAddress,
            0,
            0,
            0,
            1,
            1,
            null,
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                managerNodeId,
                "192.168.65.3",
                "Active",
                true,
                1,
                1,
                "daemon-id",
                1,
                1,
                0,
                0));
        var connector = new SwarmNodeRuntimeConnector(
            connectorFactory.Object,
            Mock.Of<IConnectorFactory<IImageConnector>>(),
            Mock.Of<IConnectorFactory<IVolumeConnector>>(),
            Mock.Of<IConnectorFactory<INetworkConnector>>(),
            Mock.Of<IEdgeAgentCommandRouter>());

        await foreach (var _ in connector.StreamContainerStatsAsync(
                           platform,
                           managerNodeId,
                           dockerContainerId,
                           fetchIntervalMs,
                           TestContext.Current.CancellationToken))
        {
        }

        Assert.NotNull(observedCommand);
        Assert.Equal(dockerContainerId, observedCommand.ContainerId);
        Assert.Equal(platformAddress, observedCommand.PlatformAddress);
        Assert.Equal(fetchIntervalMs, observedCommand.FetchIntervalMs);
    }

    private static async IAsyncEnumerable<DockerContainer> EmptyStatsStream()
    {
        await Task.CompletedTask;
        yield break;
    }

    private static async IAsyncEnumerable<EdgeAgentStreamItem> EmptyCommandStream()
    {
        await Task.CompletedTask;
        yield break;
    }

    private static Platform CreateSwarmPlatform(string managerNodeId) => new(
        "swarm",
        "http://localhost.docker",
        0,
        0,
        0,
        1,
        1,
        null,
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            managerNodeId,
            "192.168.65.3",
            "Active",
            true,
            1,
            1,
            "daemon-id",
            1,
            1,
            0,
            0));
}
