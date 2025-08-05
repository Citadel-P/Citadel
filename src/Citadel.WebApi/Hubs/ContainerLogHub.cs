using Application.Services.Abstractions;
using Application.Services.SignalR;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using static Hosting.Common.Constants;

namespace WebApi.Hubs;

public interface ITypedContainerLogHub
{
    Task SendContainerLog(string logLine);
    Task SendContainerLogsBatch(IEnumerable<string> recentLogs);
}

[Authorize]
internal sealed class ContainerLogHub(IContainerLogStreamManager streamManager, ISignalRConnectionTracker connectionTracker) : Hub<ITypedContainerLogHub>
{
    public override Task OnConnectedAsync()
    {
        connectionTracker.Connected(Context.ConnectionId, Context.GetUserId());
        return base.OnConnectedAsync();
    }

    public override Task OnDisconnectedAsync(Exception? exception)
    {
        streamManager.RemoveConnection(Context.ConnectionId);
        connectionTracker.Disconnected(Context.ConnectionId);
        return base.OnDisconnectedAsync(exception);
    }

    public async Task JoinGroup(string containerId)
    {
        await streamManager.AddSubscriber(containerId, Context.ConnectionId);
        await Groups.AddToGroupAsync(Context.ConnectionId, SignalRGroups.ContainerLogGroup(containerId));
    }

    public Task LeaveGroup(string containerId)
    {
        streamManager.RemoveSubscriber(SignalRGroups.ContainerLogGroup(containerId), Context.ConnectionId);
        return Groups.RemoveFromGroupAsync(Context.ConnectionId, SignalRGroups.ContainerLogGroup(containerId));
    }

}
