using Application.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;
using static Hosting.Common.Constants;

namespace WebApi.Hubs.Dispatchers;

internal sealed class ContainerLogHubDispatcher(IHubContext<ContainerLogHub, ITypedContainerLogHub> hubContext) : IContainerLogHubDispatcher
{
    public Task SendContainerLog(string containerId, string logLine)
        => hubContext.Clients.Group(SignalRGroups.ContainerLogGroup(containerId)).SendContainerLog(logLine);

    public Task SendContainerLogsBatch(string containerId, IEnumerable<string> recentLogs)
        => hubContext.Clients.Group(SignalRGroups.ContainerLogGroup(containerId)).SendContainerLogsBatch(recentLogs);
}