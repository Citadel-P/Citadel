using Agent.Server.Containers;
using Infrastructure.Entities;
using Infrastructure.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

internal sealed class ContainerHubDispatcher(IHubContext<ContainerHub, ITypedContainerHub> hubContext) : IContainerHubDispatcher
{
    public async Task SendContainersInfo(IEnumerable<ContainerInfo> containers) 
        => await hubContext.Clients.Group($"ContainersInfo/{containers.First().PlatformId}").ContainersInfoUpdated(ContainersInfoView.Map(containers));

    public async Task SendContainerLogs(ContainerLogReply reply, string requestId)
        => await hubContext.Clients.Group($"ContainerLogs/{reply.ContainerId}/{requestId}").ContainerLogsReceived(Mapper.Map(reply));

    public async Task SendContainerEvent(ContainerInfo container, string @event)
        => await hubContext.Clients.Group($"ContainersInfo/{container.PlatformId}").ContainerEventReceived(ContainerInfoView.Map(container), @event);
}
