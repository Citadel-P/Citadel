using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IContainerStreamManager : IStreamGroupManager
{
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
}

internal class ContainerStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IContainerStreamManager
{
    public Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainersInfo(platformId, containers);
    }

    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainersStats(platformId, containers);
    }
}
