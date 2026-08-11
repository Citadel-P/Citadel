using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.Connectors.EdgeAgentConnectors;
using Moq;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class SwarmNodeRuntimeConnectorTests
{
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
}
