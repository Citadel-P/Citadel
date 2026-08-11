using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.Connectors.EdgeAgentConnectors;
using Moq;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class SwarmNodeRuntimeConnectorTests
{
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
}
