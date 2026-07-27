using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;
using Hosting.Common;

namespace Application.Services.SignalR;

internal interface IContainerStreamManager : IStreamGroupManager
{
    bool HasStatsSubscribers(Guid platformId);
    Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers);
    Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers);
}

internal class ContainerStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IContainerStreamManager
{
    public bool HasStatsSubscribers(Guid platformId)
        => streams.ContainsKey(Constants.WellKnownSignalRGroups.ContainersGroup(platformId));

    public Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers)
    {
        if (!HasStatsSubscribers(platformId))
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainersInfo(platformId, containers);
    }

    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers)
    {
        if (!HasStatsSubscribers(platformId))
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendContainersStats(platformId, containers);
    }
}
