using Infrastructure.Entities;

namespace Infrastructure.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainerEvent(ContainerInfo container, string @event);
    Task SendContainersInfo(IEnumerable<ContainerInfo> containers);
}

