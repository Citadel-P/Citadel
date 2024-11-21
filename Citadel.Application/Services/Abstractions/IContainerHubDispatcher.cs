using Contracts.Broker.Models;
using Infrastructure.Entities;

namespace Application.Services.Abstractions;

public interface IContainerHubDispatcher
{
    Task SendContainersInfo(Guid platformId, IEnumerable<ContainerInfo> containers);
    Task SendContainerLogs(ContainerLogMessage message);
}

