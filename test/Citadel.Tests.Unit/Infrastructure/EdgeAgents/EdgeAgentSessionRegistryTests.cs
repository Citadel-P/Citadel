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

    private static EdgeAgentSession CreateSession(Guid resourceId)
        => new(
            EdgeAgentResourceType.Platform,
            resourceId,
            resourceId,
            Guid.CreateVersion7(),
            $"SHA256:{Guid.NewGuid():N}",
            Guid.CreateVersion7().ToString("D"));
}
