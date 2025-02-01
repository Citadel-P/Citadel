using Agent.Server.Containers;
using Infrastructure.Entities;

namespace Infrastructure.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainerLogs(ContainerLogReply message, string requestId);
    Task SendContainerEvent(ContainerInfo container, string @event);
    Task SendContainersInfo(IEnumerable<ContainerInfo> containers);
}

