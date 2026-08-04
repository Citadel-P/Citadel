using Domain;
using Domain.Contracts.Resources.Containers;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class DaemonEventMapperTests
{
    [Fact]
    public void LocalConnector_ShouldMapSwarmResourceEvent()
    {
        var source = new Hosting.DockerClient.Models.Platforms.DaemonResourceResult
        {
            Type = Hosting.DockerClient.EventMessageType.Node,
            Action = "update",
            ResourceId = "node-1",
            Scope = Hosting.DockerClient.EventMessageScope.Swarm
        };

        var mapped = Assert.IsType<DaemonResourceEventInfo>(source.Map());

        Assert.Equal(ContainerEventType.Node, mapped.Type);
        Assert.Equal("update", mapped.Action);
        Assert.Equal("node-1", mapped.ResourceId);
        Assert.Equal(global::Domain.Contracts.Resources.Containers.DaemonEventScope.Swarm, mapped.Scope);
    }
}
