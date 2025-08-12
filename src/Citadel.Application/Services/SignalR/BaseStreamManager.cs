using System.Collections.Concurrent;
using Application.Services.SignalR.Context;

namespace Application.Services.SignalR;

internal abstract class BaseStreamManager<TContext> where TContext : StreamContext, new()
{
    protected readonly ConcurrentDictionary<string, TContext> streams = new();
    // connectionId -> set of groupIds (as a concurrent dictionary used as a set)
    protected readonly ConcurrentDictionary<string, ConcurrentDictionary<string, byte>> connectionToGroups = new();

    // group state map (groupId -> "joining"/"joined") to protect duplicate joins across remounts/in-flight joins.
    // This is expected to be shared by provider; keeping it here makes manager self-contained:
    protected readonly ConcurrentDictionary<string, byte> groupStates = new();

    public void AddSubscriber(string groupId, string connectionId)
    {
        var context = streams.GetOrAdd(groupId, _ => new TContext());
        context.AddSubscriber(connectionId);

        // connection -> groups
        var set = connectionToGroups.GetOrAdd(connectionId, _ => new ConcurrentDictionary<string, byte>());
        set.TryAdd(groupId, 0);

        OnSubscriberAdded(groupId, connectionId);
    }

    public void RemoveSubscriber(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
        {
            // still remove mapping if present
            if (connectionToGroups.TryGetValue(connectionId, out var s))
            {
                s.TryRemove(groupId, out _);
            }
            return;
        }

        context.RemoveSubscriber(connectionId);

        // optional: start idle timer here
        if (context.IsEmpty)
        {
            streams.TryRemove(groupId, out var removed);
            if (removed is IDisposable disposable)
            {
                disposable.Dispose();
            }
        }

        if (connectionToGroups.TryGetValue(connectionId, out var set))
        {
            set.TryRemove(groupId, out _);
            if (set.IsEmpty)
            {
                connectionToGroups.TryRemove(connectionId, out _);
            }
        }

        OnSubscriberRemoved(groupId, connectionId);
    }

    public void RemoveConnection(string connectionId)
    {
        if (!connectionToGroups.TryRemove(connectionId, out var groups)) return;

        foreach (var kv in groups)
        {
            var groupId = kv.Key;
            RemoveSubscriber(groupId, connectionId);
        }
    }

    protected virtual void OnSubscriberAdded(string groupId, string connectionId) { }
    protected virtual void OnSubscriberRemoved(string groupId, string connectionId) { }

    protected static ReadOnlySpan<char> GetEntityId(ReadOnlySpan<char> groupId)
    {
        var idx = groupId.IndexOf(':');
        if (idx < 0 || idx == groupId.Length - 1)
        {
            return [];
        }

        return groupId[(idx + 1)..];
    }
}
