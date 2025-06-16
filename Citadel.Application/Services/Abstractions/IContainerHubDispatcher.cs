using Domain.Entities;

namespace Application.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainerEvent(Container container, string @event);
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
}

