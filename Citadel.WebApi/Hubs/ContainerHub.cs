using Application.Features.Platforms.Queries;
using Application.Features.Platforms.Queries.Models;
using Infrastructure.Services.Abstractions;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

public interface ITypedContainerHub
{
    Task ContainerEventReceived(ContainerView message, string @event);
    Task ContainersInfoUpdated(ContainersView message);
    Task ContainersStatsUpdated(IEnumerable<ContainerStatView> containers);
    Task ContainerLogsReceived(ContainerLogView message);
}

[Authorize]
internal sealed class ContainerHub(IMediator mediator, ISignalRConnectionTracker connectionTracker) : Hub<ITypedContainerHub>
{
    /// <inheritdoc />
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

    public async Task<ContainersView> GetContainers(Guid id)
    {
        var response = await mediator.Send(new GetContainers(new GetContainersQuery(All: true), id));
        if (response.IsSuccess(out var containers))
        {
            return ContainersView.Map(containers);
        }
        else return new ContainersView([]);
    }
}
