using Application.Features.Containers.Models;
using Infrastructure.Entities;

namespace Application.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainerLogs(ContainerLogRequest message);
    Task SendContainerEvent(ContainerInfo container, string @event);
    Task SendContainersInfo(IEnumerable<ContainerInfo> containers);
}

