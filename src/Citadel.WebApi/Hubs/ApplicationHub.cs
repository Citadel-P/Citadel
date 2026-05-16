using Application.Features.Containers.Commands;
using Application.Features.Deployments.Commands;
using Application.Features.Images.Queries;
using Application.Features.Platforms.Queries;
using Application.Services.SignalR;
using Domain;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

[Authorize]
internal sealed class ApplicationHub(IStreamSubscriptionResolver resolver, IMediator mediator) : Hub
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

    public async Task StartContainerLogs(string containerId)
    {
        await mediator.Send(new StartContainerLogs(containerId), Context.ConnectionAborted);
    }

    public async Task StartDeploymentLogs(Guid deploymentId)
    {
        await mediator.Send(new StartDeploymentLogs(deploymentId), Context.ConnectionAborted);
    }

    public async Task StartExecProcess(string containerId, string sessionId, string shell)
    {
        await mediator.Send(new StartContainerShellSession(containerId, sessionId, shell), Context.ConnectionAborted);
    }

    public async Task ResizeExec(string containerId, string sessionId, int cols, int rows) 
    {
        await mediator.Send(new ResizeContainerExecSession(containerId, sessionId, cols, rows), Context.ConnectionAborted);
    }

    public async Task SendExecInput(string containerId, string sessionId, byte[] data)
    {
        await mediator.Send(new SendContainerExecInput(containerId, sessionId, data), Context.ConnectionAborted);
    }

    public async Task StartDeploymentExecProcess(Guid deploymentId, string sessionId, string shell)
    {
        await mediator.Send(new StartDeploymentShellSession(deploymentId, sessionId, shell), Context.ConnectionAborted);
    }

    public async Task ResizeDeploymentExec(Guid deploymentId, string sessionId, int cols, int rows)
    {
        await mediator.Send(new ResizeDeploymentExecSession(deploymentId, sessionId, cols, rows), Context.ConnectionAborted);
    }

    public async Task SendDeploymentExecInput(Guid deploymentId, string sessionId, byte[] data)
    {
        await mediator.Send(new SendDeploymentExecInput(deploymentId, sessionId, data), Context.ConnectionAborted);
    }

    #endregion
}
