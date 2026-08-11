using System.Collections.Concurrent;
using Domain;
using Domain.Contracts.Interfaces;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentSessionRegistry : IEdgeAgentSessionTerminator, IEdgeAgentSessionStatus
{
    private readonly ConcurrentDictionary<EdgeAgentTarget, EdgeAgentSession> sessions = new();

    public EdgeAgentSession Register(EdgeAgentSession session)
    {
        if (TryRegister(session, out var conflict))
            return session;

        throw new InvalidOperationException(conflict);
    }

    public bool TryRegister(EdgeAgentSession session, out string? conflict)
    {
        while (true)
        {
            if (!sessions.TryGetValue(session.Target, out var previous))
            {
                if (sessions.TryAdd(session.Target, session))
                {
                    conflict = null;
                    return true;
                }

                continue;
            }

            if (session.DockerNodeId is not null && previous.AgentId != session.AgentId)
            {
                conflict = $"Swarm node '{session.DockerNodeId}' already has an authenticated Agent session.";
                return false;
            }

            if (!sessions.TryUpdate(session.Target, session, previous))
                continue;

            previous.Disconnect("Replaced by a newer Edge Agent session.");
            conflict = null;
            return true;
        }
    }

    public bool TryGet(Guid platformId, out EdgeAgentSession session)
        => TryGet(EdgeAgentResourceType.Platform, platformId, out session);

    public bool TryGet(EdgeAgentResourceType resourceType, Guid resourceId, out EdgeAgentSession session)
        => sessions.TryGetValue(new EdgeAgentTarget(resourceType, resourceId), out session!);

    public bool TryGet(Guid platformId, string dockerNodeId, out EdgeAgentSession session)
        => sessions.TryGetValue(EdgeAgentTarget.SwarmNode(platformId, dockerNodeId), out session!);

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

    public void Disconnect(Guid platformId, string dockerNodeId, string reason)
    {
        if (sessions.TryRemove(EdgeAgentTarget.SwarmNode(platformId, dockerNodeId), out var session))
            session.Disconnect(reason);
    }

    public IReadOnlyList<EdgeAgentSession> GetNodeSessions(Guid platformId)
        => sessions
            .Where(entry => entry.Key.ResourceType == EdgeAgentResourceType.Platform
                            && entry.Key.ResourceId == platformId
                            && entry.Key.DockerNodeId is not null)
            .Select(static entry => entry.Value)
            .ToArray();

    public bool IsNodeConnected(Guid platformId, string dockerNodeId)
        => TryGet(platformId, dockerNodeId, out _);

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
