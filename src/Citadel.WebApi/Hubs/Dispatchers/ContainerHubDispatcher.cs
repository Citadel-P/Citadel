using Application.Services.Abstractions;
using Domain.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using static Hosting.Common.Constants;

namespace WebApi.Hubs.Dispatchers;

internal sealed class ContainerHubDispatcher(IHubContext<ContainerHub, ITypedContainerHub> hubContext) : IContainerHubDispatcher
{
    public Task SendContainersStats(Guid platformId, IEnumerable<ContainerStat> containers)
    => hubContext.Clients.Group(SignalRGroups.ContainersGroup(platformId)).ContainersStatsUpdated(ContainerStatView.Map(containers));

    public Task SendContainersInfo(Guid platformId, IEnumerable<Container> containers) 
        => hubContext.Clients.Group(SignalRGroups.ContainersGroup(platformId)).ContainersInfoUpdated(ContainersView.Map(containers));
}
