using Gcontainers;
using Infrastructure.Entities;

namespace Application.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainerLogs(ContainerLogMessage message);
    Task SendContainerEvent(ContainerInfo container, string @event);
    Task SendContainersInfo(IEnumerable<ContainerInfo> containers);
}

