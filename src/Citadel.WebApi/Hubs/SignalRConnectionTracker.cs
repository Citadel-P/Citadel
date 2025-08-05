using System.Collections.Concurrent;
using Application.Services.Abstractions;

namespace WebApi.Hubs;

internal sealed class SignalRConnectionTracker : ISignalRConnectionTracker
{
    private readonly ConcurrentDictionary<string, string> connections = new(); // connectionId -> userId
    private readonly ConcurrentDictionary<string, HashSet<string>> userConnections = new(); // userId -> connectionIds
    private readonly ConcurrentDictionary<string, HashSet<string>> groupUsers = new(); // groupName -> userIds
    private readonly ConcurrentDictionary<string, HashSet<string>> connectionGroups = new(); // connectionId -> groupNames

    private readonly Lock @lock = new();

    public void Connected(string connectionId, string userId)
    {
        connections[connectionId] = userId;

        lock (@lock)
        {
            if (!userConnections.TryGetValue(userId, out var conns))
            {
                conns = [];
                userConnections[userId] = conns;
            }

            conns.Add(connectionId);
        }
    }

    public void Disconnected(string connectionId)
    {
        if (!connections.TryRemove(connectionId, out var userId))
            return;

        lock (@lock)
        {
            if (userConnections.TryGetValue(userId, out var conns))
            {
                conns.Remove(connectionId);

                if (conns.Count == 0)
                    userConnections.Remove(userId, out _);
            }

            // Remove from connectionGroups
            if (connectionGroups.TryGetValue(connectionId, out var groups))
            {
                foreach (var group in groups)
                {
                    if (groupUsers.TryGetValue(group, out var users))
                    {
                        users.Remove(userId);
                        if (users.Count == 0)
                            groupUsers.Remove(group, out _);
                    }
                }

                connectionGroups.Remove(connectionId, out _);
            }
        }
    }

    public void JoinGroup(string group, string connectionId, string userId)
    {
        lock (@lock)
        {
            if (!groupUsers.TryGetValue(group, out var users))
            {
                users = [];
                groupUsers[group] = users;
            }

            users.Add(userId);

            if (!connectionGroups.TryGetValue(connectionId, out var groups))
            {
                groups = [];
                connectionGroups[connectionId] = groups;
            }

            groups.Add(group);
        }
    }

    public void LeaveGroup(string group, string connectionId, string userId)
    {
        lock (@lock)
        {
            if (groupUsers.TryGetValue(group, out var users))
            {
                users.Remove(userId);
                if (users.Count == 0)
                    groupUsers.Remove(group, out _);
            }

            if (connectionGroups.TryGetValue(connectionId, out var groups))
            {
                groups.Remove(group);
                if (groups.Count == 0)
                    connectionGroups.Remove(connectionId, out _);
            }
        }
    }

    public bool TryGetUserId(string connectionId, out string userId)
        => connections.TryGetValue(connectionId, out userId);

    public IReadOnlyCollection<string> GetGroupsForConnection(string connectionId)
    {
        lock (@lock)
        {
            return connectionGroups.TryGetValue(connectionId, out var groups)
                ? [.. groups]
                : [];
        }
    }

    public bool HasAnyUsers()
    {
        lock (@lock)
            return !userConnections.IsEmpty;
    }

    public bool HasUsersInGroup(string group)
    {
        lock (@lock)
            return groupUsers.TryGetValue(group, out var users) && users.Count > 0;
    }
}
