namespace Application.Services.Abstractions;

public interface IStreamGroupManager
{
    void AddSubscriber(string groupId, string connectionId);
    void RemoveSubscriber(string groupId, string connectionId);
}
