namespace Application.Services.SignalR;

public interface IContainerLogStreamManager
{
    Task AddSubscriber(string containerId, string connectionId);
    void RemoveSubscriber(string groupId, string connectionId);
    void RemoveConnection(string connectionId);
}