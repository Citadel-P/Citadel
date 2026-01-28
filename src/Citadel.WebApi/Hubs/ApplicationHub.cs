using Application.Features.Images.Queries;
using Application.Features.Platforms.Queries;
using Application.Services.SignalR;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

[Authorize]
internal sealed class ApplicationHub(IStreamSubscriptionResolver resolver, IMediator mediator, IExecSessionManager execSessionManager) : Hub
{
    #region Overrides
    public override Task OnDisconnectedAsync(Exception? exception)
    {
        if (Context.Items.TryGetValue("GroupIds", out var obj) && obj is HashSet<string> groups)
        {
            foreach (var group in groups)
            {
                resolver.Resolve(group).RemoveConnection(Context.ConnectionId);
            }
        }
        return base.OnDisconnectedAsync(exception);
    }

    public Task JoinGroup(string groupId)
    {
        if (!Context.Items.TryGetValue("GroupIds", out var obj) || obj is not HashSet<string> groups)
        {
            groups = [];
            Context.Items["GroupIds"] = groups;
        }

        groups.Add(groupId);
        resolver.Resolve(groupId).AddSubscriber(groupId, Context.ConnectionId);
        return Groups.AddToGroupAsync(Context.ConnectionId, groupId);
    }

    public Task LeaveGroup(string groupId)
    {
        if (Context.Items.TryGetValue("GroupIds", out var obj) && obj is HashSet<string> groups)
        {
            groups.Remove(groupId);
        }
        resolver.Resolve(groupId).RemoveSubscriber(groupId, Context.ConnectionId);
        return Groups.RemoveFromGroupAsync(Context.ConnectionId, groupId);
    }

    #endregion

    #region Client Methods
    public async Task<PlatformsView> GetPlatforms()
    {
        var result = await mediator.Send(new GetPlatforms(), Context.ConnectionAborted);
        if (result.IsSuccess(out var platforms))
        {
            return PlatformsView.Map(platforms);
        }
        else return new PlatformsView([]);
    }

    public async Task<ContainersView> GetContainers(Guid id)
    {
        var response = await mediator.Send(new GetContainers(id), Context.ConnectionAborted);
        if (response.IsSuccess(out var containers))
        {
            return ContainersView.Map(containers);
        }
        else return new ContainersView([]);
    }

    public async Task<ImagesView> GetImages(Guid id)
    {
        var response = await mediator.Send(new GetAllLocalImages(id), Context.ConnectionAborted);
        if (response.IsSuccess(out var images))
        {
            return ImagesView.Map(images);
        }
        else return new ImagesView([]);
    }

    public Task StartExecProcess(string groupId, string shell) => execSessionManager.StartExecProcess(groupId, shell, Context.ConnectionAborted);

    public Task ResizeExec(string groupId, int cols, int rows, string shell) => execSessionManager.ResizeAsync(groupId, cols, rows, shell, Context.ConnectionAborted);

    public Task SendExecInput(string groupId, byte[] data) => execSessionManager.SendInputAsync(groupId, data, Context.ConnectionAborted);
    #endregion
}
