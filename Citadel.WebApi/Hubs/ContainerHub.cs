using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

public interface ITypedContainerHub
{
    Task ContainerEventReceived(ContainerInfoView message, string @event);
    Task ContainersInfoUpdated(ContainersInfoView message);
    Task ContainerLogsReceived(ContainerLogView message);
}

[Authorize]
internal sealed class ContainerHub : Hub<ITypedContainerHub>
{
    /// <summary>
    /// Must be called from client side in order to join a group
    /// </summary>
    public async Task JoinGroup(string groupName)
        => await Groups.AddToGroupAsync(Context.ConnectionId, groupName);
    
    /// <summary>
    /// Must be called from client side to leave a group
    /// </summary>
    public async Task LeaveGroup(string groupName) 
        => await Groups.RemoveFromGroupAsync(Context.ConnectionId, groupName);
}
