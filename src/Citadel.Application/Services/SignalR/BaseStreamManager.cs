using System.Collections.Concurrent;
using Application.Services.SignalR.Context;

namespace Application.Services.SignalR;

internal abstract class BaseStreamManager<TContext> where TContext : StreamContext, new()
{
    protected readonly ConcurrentDictionary<string, TContext> streams = new();
    protected readonly ConcurrentDictionary<string, ConcurrentDictionary<string, byte>> connectionToGroups = new();

    public void AddSubscriber(string groupId, string connectionId)
    {
        var context = streams.GetOrAdd(groupId, _ => new TContext());
        context.AddSubscriber(connectionId);

        var set = connectionToGroups.GetOrAdd(connectionId, _ => new ConcurrentDictionary<string, byte>());
        set.TryAdd(groupId, 0);

        OnSubscriberAdded(groupId, connectionId);
    }

    public void RemoveSubscriber(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
        {
            if (connectionToGroups.TryGetValue(connectionId, out var s))
                s.TryRemove(groupId, out _);

            return;
        }

        context.RemoveSubscriber(connectionId);

        if (context.IsEmpty)
        {
            streams.TryRemove(groupId, out var removed);
            if (removed is IDisposable disposable)
                disposable.Dispose();
        }

        if (connectionToGroups.TryGetValue(connectionId, out var set))
        {
            set.TryRemove(groupId, out _);
            if (set.IsEmpty)
                connectionToGroups.TryRemove(connectionId, out _);
        }

        OnSubscriberRemoved(groupId, connectionId);
    }

    public void RemoveConnection(string connectionId)
    {
        if (!connectionToGroups.TryRemove(connectionId, out var groups)) return;

        foreach (var kv in groups)
            RemoveSubscriber(kv.Key, connectionId);
    }

    protected virtual void OnSubscriberAdded(string groupId, string connectionId) { }
    protected virtual void OnSubscriberRemoved(string groupId, string connectionId) { }

    protected static string GetNormalizedIdFromGroup(ReadOnlySpan<char> groupId)
    {
        var idx = groupId.IndexOf(':');
        if ((uint)idx >= (uint)(groupId.Length - 1))
            return string.Empty;

        return NormalizeDockerId(groupId[(idx + 1)..]);
    }

    protected static string NormalizeDockerId(ReadOnlySpan<char> dockerId)
    {
        if (dockerId.IsEmpty)
            return string.Empty;

        if (dockerId.Length > 12)
            dockerId = dockerId[..12];

        return dockerId.ToString().ToLowerInvariant();
    }
}
