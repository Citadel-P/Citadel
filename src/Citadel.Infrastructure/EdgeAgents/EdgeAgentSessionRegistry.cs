using System.Collections.Concurrent;
using Domain;
using Domain.Contracts.Interfaces;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentSessionRegistry : IEdgeAgentSessionTerminator
{
    private readonly ConcurrentDictionary<EdgeAgentTarget, EdgeAgentSession> sessions = new();

    public EdgeAgentSession Register(EdgeAgentSession session)
    {
        sessions.AddOrUpdate(
            session.Target,
            session,
            (_, previous) =>
            {
                previous.Disconnect("Replaced by a newer Edge Agent session.");
                return session;
            });

        return session;
    }

    public bool TryGet(Guid platformId, out EdgeAgentSession session)
        => TryGet(EdgeAgentResourceType.Platform, platformId, out session);

    public bool TryGet(EdgeAgentResourceType resourceType, Guid resourceId, out EdgeAgentSession session)
        => sessions.TryGetValue(new EdgeAgentTarget(resourceType, resourceId), out session!);

    public void Disconnect(Guid platformId, string reason)
    {
        Disconnect(EdgeAgentResourceType.Platform, platformId, reason);
    }

    public void Disconnect(EdgeAgentResourceType resourceType, Guid resourceId, string reason)
    {
        if (sessions.TryRemove(new EdgeAgentTarget(resourceType, resourceId), out var session))
        {
            session.Disconnect(reason);
        }
    }

    public void Remove(EdgeAgentSession session)
    {
        if (sessions.TryGetValue(session.Target, out var current) &&
            ReferenceEquals(current, session))
        {
            sessions.TryRemove(session.Target, out _);
        }
    }
}
