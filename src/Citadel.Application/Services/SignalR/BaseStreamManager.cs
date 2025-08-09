using System.Collections.Concurrent;
using Application.Services.SignalR.Context;

namespace Application.Services.SignalR;

internal abstract class BaseStreamManager<TContext> where TContext : StreamContext, new()
{
    protected readonly ConcurrentDictionary<string, TContext> streams = [];

    public void AddSubscriber(string groupId, string connectionId)
    {
        var context = streams.GetOrAdd(groupId, _ => new TContext());
        context.AddSubscriber(connectionId);

        OnSubscriberAdded(groupId, connectionId);
    }

    public void RemoveSubscriber(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
        {
            return;
        }

        context.RemoveSubscriber(connectionId);
        if (context.IsEmpty)
        {
            streams.TryRemove(groupId, out _);
        }

        OnSubscriberRemoved(groupId, connectionId);
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
