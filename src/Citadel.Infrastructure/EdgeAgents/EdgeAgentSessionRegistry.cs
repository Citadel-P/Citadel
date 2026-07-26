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

    public bool RemoveAndShouldMarkDisconnected(EdgeAgentSession session)
    {
        var removed = ((ICollection<KeyValuePair<EdgeAgentTarget, EdgeAgentSession>>)sessions)
            .Remove(new KeyValuePair<EdgeAgentTarget, EdgeAgentSession>(
                session.Target,
                session));
        if (removed)
        {
            return true;
        }

        // A missing entry was terminated explicitly, such as by revocation.
        // A different entry is a replacement session and owns the target state.
        return !sessions.ContainsKey(session.Target);
    }
}
