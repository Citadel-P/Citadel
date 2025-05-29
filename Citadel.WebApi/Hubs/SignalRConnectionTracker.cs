using System.Collections.Concurrent;
using Infrastructure.Services.Abstractions;

namespace WebApi.Hubs;

internal sealed class SignalRConnectionTracker : ISignalRConnectionTracker
{
    private readonly ConcurrentDictionary<string, string> connections = new(); 
    private readonly ConcurrentDictionary<string, HashSet<string>> userConnections = new(); 
    private readonly ConcurrentDictionary<string, HashSet<string>> groupUsers = new();

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
                {
                    userConnections.Remove(userId, out _);

                    // Remove user from all groups
                    foreach (var (group, users) in groupUsers)
                    {
                        users.Remove(userId);
                        if (users.Count == 0)
                            groupUsers.Remove(group, out _);
                    }
                }
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
