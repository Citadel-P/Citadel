using Application.Features.Platforms.Queries;
using Application.Features.Platforms.Queries.Models;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Hubs;

public interface ITypedContainerHub
{
    Task ContainerEventReceived(ContainerInfoView message, string @event);
    Task ContainersInfoUpdated(ContainersInfoView message);
    Task ContainersStatsUpdated(IEnumerable<ContainerStatView> containers);
    Task ContainerLogsReceived(ContainerLogView message);
}

[Authorize]
internal sealed class ContainerHub(IMediator mediator) : Hub<ITypedContainerHub>
{
    /// <summary>
    /// Must be called from client side in order to join a group
    /// </summary>
    public Task JoinGroup(string groupName)
        => Groups.AddToGroupAsync(Context.ConnectionId, groupName);
    
    /// <summary>
    /// Must be called from client side to leave a group
    /// </summary>
    public Task LeaveGroup(string groupName) 
        => Groups.RemoveFromGroupAsync(Context.ConnectionId, groupName);

    public async Task<ContainersInfoView> GetContainers(Guid id)
    {
        var response = await mediator.Send(new GetContainers(new GetContainersQuery(All: true), id));
        if (response.IsSuccess(out var containers))
        {
            return ContainersInfoView.Map(containers);
        }
        else return new ContainersInfoView([]);
    }
}
