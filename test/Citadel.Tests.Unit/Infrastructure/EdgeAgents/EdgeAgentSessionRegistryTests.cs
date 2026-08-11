using Domain;
using Infrastructure.EdgeAgents;

namespace Tests.Unit.Infrastructure.EdgeAgents;

public sealed class EdgeAgentSessionRegistryTests
{
    [Fact]
    public void RemoveCurrentSession_ShouldMarkTargetDisconnected()
    {
        var registry = new EdgeAgentSessionRegistry();
        var session = CreateSession(Guid.CreateVersion7());
        registry.Register(session);

        var shouldMarkDisconnected =
            registry.RemoveAndShouldMarkDisconnected(session);

        Assert.True(shouldMarkDisconnected);
        Assert.False(registry.TryGet(
            session.ResourceType,
            session.ResourceId,
            out _));
    }

    [Fact]
    public void RemoveReplacedSession_ShouldKeepReplacementConnected()
    {
        var registry = new EdgeAgentSessionRegistry();
        var resourceId = Guid.CreateVersion7();
        var replaced = CreateSession(resourceId);
        var replacement = CreateSession(resourceId);
        registry.Register(replaced);
        registry.Register(replacement);

        var shouldMarkDisconnected =
            registry.RemoveAndShouldMarkDisconnected(replaced);

        Assert.False(shouldMarkDisconnected);
        Assert.True(registry.TryGet(
            replacement.ResourceType,
            replacement.ResourceId,
            out var current));
        Assert.Same(replacement, current);
    }

    [Fact]
    public void RemoveTerminatedSession_ShouldAllowRevocationCleanup()
    {
        var registry = new EdgeAgentSessionRegistry();
        var session = CreateSession(Guid.CreateVersion7());
        registry.Register(session);
        registry.Disconnect(
            session.ResourceType,
            session.ResourceId,
            "revoked");

        var shouldMarkDisconnected =
            registry.RemoveAndShouldMarkDisconnected(session);

        Assert.True(shouldMarkDisconnected);
    }

    [Fact]
    public void RegisterSwarmNodes_ShouldKeepIndependentSessionsForOnePlatform()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var first = CreateNodeSession(platformId, "node-1", Guid.CreateVersion7());
        var second = CreateNodeSession(platformId, "node-2", Guid.CreateVersion7());

        Assert.True(registry.TryRegister(first, out var firstConflict), firstConflict);
        Assert.True(registry.TryRegister(second, out var secondConflict), secondConflict);

        Assert.True(registry.TryGet(platformId, "node-1", out var firstCurrent));
        Assert.True(registry.TryGet(platformId, "node-2", out var secondCurrent));
        Assert.Same(first, firstCurrent);
        Assert.Same(second, secondCurrent);
        Assert.Equal(2, registry.GetNodeSessions(platformId).Count);
    }

    [Fact]
    public void ReconnectSwarmNode_ShouldReplaceOnlyThatNodeAndIgnoreLateDisconnect()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var agentId = Guid.CreateVersion7();
        var previous = CreateNodeSession(platformId, "node-1", agentId);
        var replacement = CreateNodeSession(platformId, "node-1", agentId);
        var other = CreateNodeSession(platformId, "node-2", Guid.CreateVersion7());
        registry.Register(previous);
        registry.Register(other);

        Assert.True(registry.TryRegister(replacement, out var conflict), conflict);
        Assert.False(registry.RemoveAndShouldMarkDisconnected(previous));

        Assert.True(registry.TryGet(platformId, "node-1", out var current));
        Assert.Same(replacement, current);
        Assert.True(registry.TryGet(platformId, "node-2", out var otherCurrent));
        Assert.Same(other, otherCurrent);
    }

    [Fact]
    public void RegisterSwarmNode_ShouldRejectDifferentAgentForConnectedNode()
    {
        var registry = new EdgeAgentSessionRegistry();
        var platformId = Guid.CreateVersion7();
        var current = CreateNodeSession(platformId, "node-1", Guid.CreateVersion7());
        var conflicting = CreateNodeSession(platformId, "node-1", Guid.CreateVersion7());
        registry.Register(current);

        Assert.False(registry.TryRegister(conflicting, out var conflict));
        Assert.Contains("already has an authenticated Agent session", conflict);
        Assert.True(registry.TryGet(platformId, "node-1", out var retained));
        Assert.Same(current, retained);
    }

    private static EdgeAgentSession CreateSession(Guid resourceId)
        => new(
            EdgeAgentResourceType.Platform,
            resourceId,
            resourceId,
            Guid.CreateVersion7(),
            $"SHA256:{Guid.NewGuid():N}",
            Guid.CreateVersion7().ToString("D"));

    private static EdgeAgentSession CreateNodeSession(Guid platformId, string dockerNodeId, Guid agentId)
        => new(
            EdgeAgentResourceType.Platform,
            platformId,
            platformId,
            agentId,
            $"SHA256:{Guid.NewGuid():N}",
            Guid.CreateVersion7().ToString("D"),
            EdgeAgentProfile.SwarmNode,
            dockerNodeId);
}
