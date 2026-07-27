using System.Collections.Concurrent;
using Application.Services.SignalR.Context;

namespace Application.Services.SignalR;

internal abstract class BaseStreamManager<TContext> where TContext : StreamContext, new()
{
    protected readonly ConcurrentDictionary<string, TContext> streams = new();
    protected readonly ConcurrentDictionary<string, ConcurrentDictionary<string, byte>> connectionToGroups = new();
    private readonly Lock subscriptionGate = new();

    public void AddSubscriber(string groupId, string connectionId)
    {
        using (subscriptionGate.EnterScope())
        {
            var context = streams.GetOrAdd(groupId, _ => new TContext());
            context.AddSubscriber(connectionId);

            var set = connectionToGroups.GetOrAdd(connectionId, _ => new ConcurrentDictionary<string, byte>());
            set.TryAdd(groupId, 0);

            OnSubscriberAdded(groupId, connectionId);
        }
    }

    public void RemoveSubscriber(string groupId, string connectionId)
    {
        IDisposable? disposable = null;
        using (subscriptionGate.EnterScope())
        {
            disposable = RemoveSubscriberUnsafe(groupId, connectionId, updateConnectionMapping: true);
        }

        disposable?.Dispose();
    }

    public void RemoveConnection(string connectionId)
    {
        List<IDisposable>? disposables = null;
        using (subscriptionGate.EnterScope())
        {
            if (!connectionToGroups.TryRemove(connectionId, out var groups))
                return;

            foreach (var groupId in groups.Keys)
            {
                if (RemoveSubscriberUnsafe(groupId, connectionId, updateConnectionMapping: false) is { } disposable)
                    (disposables ??= []).Add(disposable);
            }
        }

        if (disposables is not null)
        {
            foreach (var disposable in disposables)
                disposable.Dispose();
        }
    }

    protected bool TryUseStream(string groupId, Action<TContext> action)
    {
        using (subscriptionGate.EnterScope())
        {
            if (!streams.TryGetValue(groupId, out var context) || context.IsEmpty)
                return false;

            action(context);
            return true;
        }
    }

    private IDisposable? RemoveSubscriberUnsafe(
        string groupId,
        string connectionId,
        bool updateConnectionMapping)
    {
        IDisposable? disposable = null;
        if (streams.TryGetValue(groupId, out var context))
        {
            context.RemoveSubscriber(connectionId);

            if (context.IsEmpty)
            {
                streams.TryRemove(groupId, out _);
                disposable = context as IDisposable;
            }

            OnSubscriberRemoved(groupId, connectionId);
        }

        if (updateConnectionMapping &&
            connectionToGroups.TryGetValue(connectionId, out var groups))
        {
            groups.TryRemove(groupId, out _);
            if (groups.IsEmpty)
                connectionToGroups.TryRemove(connectionId, out _);
        }

        return disposable;
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
