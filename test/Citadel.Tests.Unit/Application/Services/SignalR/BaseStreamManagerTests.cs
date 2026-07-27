using Application.Services.SignalR;
using Application.Services.SignalR.Context;

namespace Tests.Unit.Application.Services.SignalR;

public class BaseStreamManagerTests
{
    // Minimal concrete StreamContext for testing that exposes subscriber count.
    private class TestContext : StreamContext
    {
        public int SubscriberCount
        {
            get
            {
                using (@lock.EnterScope())
                {
                    return subscribers.Count;
                }
            }
        }

        public override void AddSubscriber(string connectionId)
        {
            using (@lock.EnterScope())
            {
                subscribers.Add(connectionId);
            }
        }

        public override void RemoveSubscriber(string connectionId)
        {
            using (@lock.EnterScope())
            {
                subscribers.Remove(connectionId);
            }
        }
    }

    // Concrete manager to expose internal state and hooks for assertions.
    private class TestManager : BaseStreamManager<TestContext>
    {
        public int AddedHookCount { get; private set; }
        public int RemovedHookCount { get; private set; }

        protected override void OnSubscriberAdded(string groupId, string connectionId) => AddedHookCount++;
        protected override void OnSubscriberRemoved(string groupId, string connectionId) => RemovedHookCount++;

        public bool HasStream(string groupId) => streams.ContainsKey(groupId);
        public int StreamCount => streams.Count;
        public int GetSubscriberCount(string groupId) => streams.TryGetValue(groupId, out var ctx) ? ctx.SubscriberCount : -1;

        public bool HasConnection(string connectionId) => connectionToGroups.ContainsKey(connectionId);
        public IReadOnlyCollection<string> GetConnectionGroups(string connectionId)
            => connectionToGroups.TryGetValue(connectionId, out var set) ? set.Keys.ToList() : Array.Empty<string>();

        public string GetEntityIdPublic(string groupId) => new([.. GetNormalizedIdFromGroup(groupId)]);
        public bool TryUsePublic(string groupId, Action<TestContext> action) => TryUseStream(groupId, action);
    }

    [Fact]
    public void AddSubscriber_CreatesStreamAndConnectionMapping_AndCallsHook()
    {
        var mgr = new TestManager();

        mgr.AddSubscriber("group:42", "conn-1");

        Assert.True(mgr.HasStream("group:42"));
        Assert.Equal(1, mgr.GetSubscriberCount("group:42"));
        Assert.True(mgr.HasConnection("conn-1"));
        Assert.Contains("group:42", mgr.GetConnectionGroups("conn-1"));
        Assert.Equal(1, mgr.AddedHookCount);
    }

    [Fact]
    public void RemoveSubscriber_RemovesSubscriberAndCleansUpStream_AndCallsHook()
    {
        var mgr = new TestManager();

        mgr.AddSubscriber("g1", "c1");
        Assert.True(mgr.HasStream("g1"));
        Assert.True(mgr.HasConnection("c1"));

        mgr.RemoveSubscriber("g1", "c1");

        // stream should be removed because it became empty
        Assert.False(mgr.HasStream("g1"));
        // connection mapping should be removed
        Assert.False(mgr.HasConnection("c1"));
        Assert.Equal(1, mgr.RemovedHookCount);
    }

    [Fact]
    public void RemoveConnection_RemovesAllSubscriptionsForConnection()
    {
        var mgr = new TestManager();

        mgr.AddSubscriber("g1", "cA");
        mgr.AddSubscriber("g2", "cA");
        Assert.True(mgr.HasStream("g1"));
        Assert.True(mgr.HasStream("g2"));
        Assert.True(mgr.HasConnection("cA"));

        mgr.RemoveConnection("cA");

        // both groups should be removed (each had only that single subscriber)
        Assert.False(mgr.HasStream("g1"));
        Assert.False(mgr.HasStream("g2"));
        Assert.False(mgr.HasConnection("cA"));

        // two removals expected (one per group)
        Assert.Equal(2, mgr.RemovedHookCount);
    }

    [Fact]
    public void GetEntityId_ReturnsEntityPortionOrEmpty()
    {
        var mgr = new TestManager();

        var withEntity = mgr.GetEntityIdPublic("prefix:entityId");
        Assert.Equal("entityid", withEntity);

        var noColon = mgr.GetEntityIdPublic("no-colon");
        Assert.Equal(string.Empty, noColon);

        var trailingColon = mgr.GetEntityIdPublic("ends-with-colon:");
        Assert.Equal(string.Empty, trailingColon);
    }

    [Fact]
    public void TryUseStream_DoesNotCreateAContextWithoutASubscriber()
    {
        var mgr = new TestManager();
        var used = mgr.TryUsePublic("group:42", _ => throw new InvalidOperationException());

        Assert.False(used);
        Assert.Equal(0, mgr.StreamCount);
    }

    [Fact]
    public async Task ConcurrentSubscriptionChanges_LeaveNoOrphanedStreams()
    {
        var mgr = new TestManager();
        var cancellationToken = global::Xunit.TestContext.Current.CancellationToken;
        var operations = Enumerable.Range(0, 500)
            .Select(index => Task.Run(() =>
            {
                var connectionId = $"connection-{index}";
                mgr.AddSubscriber("group:42", connectionId);
                mgr.RemoveSubscriber("group:42", connectionId);
            }, cancellationToken));

        await Task.WhenAll(operations);

        Assert.Equal(0, mgr.StreamCount);
        Assert.Empty(mgr.GetConnectionGroups("connection-0"));
    }
}
