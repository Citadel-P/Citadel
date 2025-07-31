using Application.Services;
using Domain.Contracts.Resources.Containers;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

public interface ITypedContainerInfoHub
{
    Task ReceiveContainerInfo(DockerContainer container);
}

[Authorize]
internal sealed class ContainerInfoHub(IContainerInfoStreamManager streamManager) : Hub<ITypedContainerInfoHub>
{
    public static string GroupName(string id) => $"container-{id}";

    public async Task Subscribe(string containerId)
    {
        await Groups.AddToGroupAsync(Context.ConnectionId, GroupName(containerId));
        streamManager.AddSubscriber(containerId, Context.ConnectionId);
    }

    public async Task Unsubscribe(string containerId)
    {
        await Groups.RemoveFromGroupAsync(Context.ConnectionId, GroupName(containerId));
        streamManager.RemoveSubscriber(containerId, Context.ConnectionId);
    }

}
