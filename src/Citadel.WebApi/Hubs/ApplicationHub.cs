using Application.Features.Platforms.Queries;
using Application.Services.SignalR;
using Domain.Contracts.Resources.Containers;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedApplicationHub
{
    #region Docker Daemon Events
    Task ContainerEventReceived(ContainerView message, string @event);
    #endregion

    #region Container Info
    Task ReceiveContainerInfo(DockerContainer container);
    #endregion

    #region Container Logs
    Task SendContainerLog(string logLine);
    Task SendContainerLogsBatch(IEnumerable<string> recentLogs);
    #endregion

    #region Containers
    Task ContainersInfoUpdated(ContainersView message);
    Task ContainersStatsUpdated(IEnumerable<ContainerStatView> containers);
    #endregion

    #region Platforms
    Task PlatformsUpdated(IEnumerable<PlatformView> platforms);
    Task PlatformStatsUpdated(PlatformStatsBatchView platform);
    Task PlatformUpdated(PlatformView platform);
    Task PlatformsDeleted(Guid platformId);
    #endregion
}

[Authorize]
internal sealed class ApplicationHub(IStreamSubscriptionResolver resolver, IMediator mediator) : Hub<ITypedApplicationHub>
{
    #region Overrides
    public override Task OnDisconnectedAsync(Exception? exception)
    {
        var groupId = Context.Items["GroupId"]?.ToString() ?? "";
        if (!string.IsNullOrEmpty(groupId))
        {
            resolver.Resolve(groupId).RemoveSubscriber(groupId, Context.ConnectionId);
        }

        return base.OnDisconnectedAsync(exception);
    }

    public Task JoinGroup(string groupId)
    {
        Context.Items["GroupId"] = groupId;
        resolver.Resolve(groupId).AddSubscriber(groupId, Context.ConnectionId);
        return Groups.AddToGroupAsync(Context.ConnectionId, groupId);
    }

    public Task LeaveGroup(string groupId)
    {
        resolver.Resolve(groupId).RemoveSubscriber(groupId, Context.ConnectionId);
        return Groups.RemoveFromGroupAsync(Context.ConnectionId, groupId);
    }

    #endregion

    #region Client Methods
    public async Task<PlatformsView> GetPlatforms()
    {
        var result = await mediator.Send(new GetPlatforms());
        if (result.IsSuccess(out var platforms))
        {
            return PlatformsView.Map(platforms);
        }
        else return new PlatformsView([]);
    }

    public async Task<ContainersView> GetContainers(Guid id)
    {
        var response = await mediator.Send(new GetContainers(id));
        if (response.IsSuccess(out var containers))
        {
            return ContainersView.Map(containers);
        }
        else return new ContainersView([]);
    }
    #endregion
}
