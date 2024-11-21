using Application.Services.Abstractions;
using Contracts.Broker.Models;
using Infrastructure.Entities;
using Microsoft.AspNetCore.SignalR;
using WebApi.Controllers.V1.Resources;
using WebApi.Controllers.V1.Resources.Containers;

namespace WebApi.Hubs;

internal sealed class ContainerHubDispatcher(IHubContext<ContainerHub, ITypedContainerHub> hubContext) : IContainerHubDispatcher
{
    public async Task SendContainersInfo(Guid platformId, IEnumerable<ContainerInfo> containers) 
        => await hubContext.Clients.Group($"ContainersInfo/{platformId}").ContainersInfoUpdated(ContainersInfoView.Map(containers));

    public async Task SendContainerLogs(ContainerLogMessage message)
        => await hubContext.Clients.Group($"ContainerLogs/{message.ContainerId}/{message.RequestId}").ContainerLogsReceived(Mapper.Map(message));
}