using System.Collections.Concurrent;
using Domain.Contracts.Interfaces;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgeAgentSessionRegistry : IEdgeAgentSessionTerminator
{
    private readonly ConcurrentDictionary<Guid, EdgeAgentSession> sessions = new();

    public EdgeAgentSession Register(EdgeAgentSession session)
    {
        sessions.AddOrUpdate(
            session.PlatformId,
            session,
            (_, previous) =>
            {
                previous.Disconnect("Replaced by a newer Edge Agent session.");
                return session;
            });

        return session;
    }

    public bool TryGet(Guid platformId, out EdgeAgentSession session)
        => sessions.TryGetValue(platformId, out session!);

    public void Disconnect(Guid platformId, string reason)
    {
        if (sessions.TryRemove(platformId, out var session))
        {
            session.Disconnect(reason);
        }
    }

    public void Remove(EdgeAgentSession session)
    {
        if (sessions.TryGetValue(session.PlatformId, out var current) &&
            ReferenceEquals(current, session))
        {
            sessions.TryRemove(session.PlatformId, out _);
        }
    }
}
