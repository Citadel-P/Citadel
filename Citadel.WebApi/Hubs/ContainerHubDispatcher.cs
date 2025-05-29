using Infrastructure.Entities;
using Infrastructure.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

internal sealed class ContainerHubDispatcher(IHubContext<ContainerHub, ITypedContainerHub> hubContext) : IContainerHubDispatcher
{
    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers)
    => hubContext.Clients.Group($"ContainersInfo/{platformId}").ContainersStatsUpdated(ContainerStatView.Map(containers));

    public Task SendContainersInfo(IEnumerable<ContainerInfo> containers) 
        => hubContext.Clients.Group($"ContainersInfo/{containers.First().PlatformId}").ContainersInfoUpdated(ContainersInfoView.Map(containers));

    public Task SendContainerEvent(ContainerInfo container, string @event)
        => hubContext.Clients.Group($"ContainersInfo/{container.PlatformId}").ContainerEventReceived(ContainerInfoView.Map(container), @event);
}
