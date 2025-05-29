namespace Infrastructure.Services.Abstractions;

/// <summary>
/// In-memory signalR connection tracker
/// </summary>
public interface ISignalRConnectionTracker
{
    void Connected(string connectionId, string userId);
    void Disconnected(string connectionId);

    void JoinGroup(string group, string connectionId, string userId);
    void LeaveGroup(string group, string connectionId, string userId);

    bool HasAnyUsers();
    bool HasUsersInGroup(string group);
}
