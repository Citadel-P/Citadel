using System.Diagnostics.CodeAnalysis;
using Application.Services.Abstractions;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

internal abstract class HubBase<[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)] T>(ISignalRConnectionTracker connectionTracker) : Hub<T> where T : class
{
    public override Task OnConnectedAsync()
    {
        connectionTracker.Connected(Context.ConnectionId, Context.GetUserId());
        return base.OnConnectedAsync();
    }

    /// <inheritdoc />
    public override Task OnDisconnectedAsync(Exception? exception)
    {
        connectionTracker.Disconnected(Context.ConnectionId);
        return base.OnDisconnectedAsync(exception);
    }

    /// <summary>
    /// Must be called from client side in order to join a group
    /// </summary>
    public Task JoinGroup(string groupName)
    {
        connectionTracker.JoinGroup(groupName, Context.ConnectionId, Context.GetUserId());
        return Groups.AddToGroupAsync(Context.ConnectionId, groupName);
    }

    /// <summary>
    /// Must be called from client side to leave a group
    /// </summary>
    public Task LeaveGroup(string groupName)
    {
        connectionTracker.LeaveGroup(groupName, Context.ConnectionId, Context.GetUserId());
        return Groups.RemoveFromGroupAsync(Context.ConnectionId, groupName);
    }
}
