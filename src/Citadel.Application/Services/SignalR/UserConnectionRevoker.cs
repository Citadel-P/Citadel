namespace Application.Services.SignalR;

public interface IUserConnectionRevoker
{
    void Register(Guid userId, string connectionId, Action abort);
    void Unregister(Guid userId, string connectionId);
    void RevokeUsers(IEnumerable<Guid> userIds);
}

internal sealed class UserConnectionRevoker : IUserConnectionRevoker
{
    private readonly object sync = new();
    private readonly Dictionary<Guid, Dictionary<string, Action>> connections = [];

    public void Register(Guid userId, string connectionId, Action abort)
    {
        lock (sync)
        {
            if (!connections.TryGetValue(userId, out var userConnections))
            {
                userConnections = [];
                connections.Add(userId, userConnections);
            }

            userConnections[connectionId] = abort;
        }
    }

    public void Unregister(Guid userId, string connectionId)
    {
        lock (sync)
        {
            if (!connections.TryGetValue(userId, out var userConnections))
                return;

            userConnections.Remove(connectionId);
            if (userConnections.Count == 0)
                connections.Remove(userId);
        }
    }

    public void RevokeUsers(IEnumerable<Guid> userIds)
    {
        List<Action> aborts = [];

        lock (sync)
        {
            foreach (var userId in userIds.Distinct())
            {
                if (!connections.Remove(userId, out var userConnections))
                    continue;

                aborts.AddRange(userConnections.Values);
            }
        }

        foreach (var abort in aborts)
        {
            try
            {
                abort();
            }
            catch
            {
            }
        }
    }
}
