using Application.Features.Platforms.Queries;
using Infrastructure.Services.Abstractions;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformsUpdated(IEnumerable<PlatformView> platforms);
    Task PlatformStatsUpdated(PlatformStatsBatchView platform);
    Task PlatformUpdated(PlatformView platform);
    Task PlatformsDeleted(Guid platformId);
}

[Authorize]
internal sealed class PlatformHub(IMediator mediator, ISignalRConnectionTracker connectionTracker) : Hub<ITypedPlatformHub>
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

    public async Task<PlatformsView> GetPlatforms()
    {
        var result = await mediator.Send(new GetPlatforms());
        if (result.IsSuccess(out var platforms))
        {
            return PlatformsView.Map(platforms);
        }
        else return new PlatformsView([]);
    }
}
