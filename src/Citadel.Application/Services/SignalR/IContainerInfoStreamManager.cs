namespace Application.Services.SignalR;

/// <summary>
/// Manages and start/stop a stream based on number of active subscribers
/// </summary>
public interface IContainerInfoStreamManager
{
    void AddSubscriber(string containerId, string connectionId);
    void RemoveSubscriber(string groupId, string connectionId);
    void RemoveConnection(string connectionId);
}
