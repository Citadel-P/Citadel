using Application.Features.Containers.Commands;
using Application.Features.Deployments.Commands;
using Application.Features.Stacks.Commands;
using Application.Features.Swarm.Commands;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Extensions;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

[Authorize]
internal sealed class ApplicationHub(
    IStreamSubscriptionResolver resolver,
    ISignalRGroupAuthorizationService groupAuthorizationService,
    IUserConnectionRevoker connectionRevoker,
    IMediator mediator) : Hub
{
    #region Overrides
    public override async Task OnConnectedAsync()
    {
        var userId = Context.User!.GetUserId();
        Context.Items["UserId"] = userId;
        connectionRevoker.Register(userId, Context.ConnectionId, Context.Abort);
        try
        {
            await base.OnConnectedAsync();
        }
        catch
        {
            connectionRevoker.Unregister(userId, Context.ConnectionId);
            throw;
        }
    }

    public override Task OnDisconnectedAsync(Exception? exception)
    {
        if (Context.Items.TryGetValue("GroupIds", out var obj) && obj is HashSet<string> groups)
        {
            foreach (var group in groups)
            {
                resolver.Resolve(group).RemoveConnection(Context.ConnectionId);
            }
        }

        if (Context.Items.TryGetValue("UserId", out var userIdValue) && userIdValue is Guid userId)
            connectionRevoker.Unregister(userId, Context.ConnectionId);

        return base.OnDisconnectedAsync(exception);
    }

    public async Task JoinGroup(string groupId)
    {
        if (!await groupAuthorizationService.CanJoinAsync(
                Context.User!,
                groupId,
                Context.ConnectionAborted))
            throw new HubException("Not authorized to join this group.");

        if (!Context.Items.TryGetValue("GroupIds", out var obj) || obj is not HashSet<string> groups)
        {
            groups = [];
            Context.Items["GroupIds"] = groups;
        }

        if (!groups.Add(groupId))
            return;

        resolver.Resolve(groupId).AddSubscriber(groupId, Context.ConnectionId);
        try
        {
            await Groups.AddToGroupAsync(
                Context.ConnectionId,
                GetSignalRGroupId(groupId),
                Context.ConnectionAborted);
        }
        catch
        {
            groups.Remove(groupId);
            resolver.Resolve(groupId).RemoveSubscriber(groupId, Context.ConnectionId);
            throw;
        }
    }

    public async Task LeaveGroup(string groupId)
    {
        if (!Context.Items.TryGetValue("GroupIds", out var obj) ||
            obj is not HashSet<string> groups ||
            !groups.Remove(groupId))
            return;

        resolver.Resolve(groupId).RemoveSubscriber(groupId, Context.ConnectionId);
        await Groups.RemoveFromGroupAsync(
            Context.ConnectionId,
            GetSignalRGroupId(groupId),
            Context.ConnectionAborted);
    }

    #endregion

    #region Client Methods

    public async Task StartContainerLogs(string containerId)
    {
        await mediator.Send(new StartContainerLogs(containerId), Context.ConnectionAborted);
    }

    public async Task StartDeploymentLogs(Guid deploymentId)
    {
        await mediator.Send(new StartDeploymentLogs(deploymentId), Context.ConnectionAborted);
    }

    public async Task StartStackLogs(Guid stackId)
    {
        await mediator.Send(new StartStackLogs(stackId), Context.ConnectionAborted);
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

    public async Task StartSwarmTaskExecProcess(Guid platformId, string taskId, string sessionId, string shell)
    {
        await mediator.Send(new StartSwarmTaskShellSession(platformId, taskId, sessionId, shell), Context.ConnectionAborted);
    }

    public async Task ResizeSwarmTaskExec(Guid platformId, string taskId, string sessionId, int cols, int rows)
    {
        await mediator.Send(new ResizeSwarmTaskExecSession(platformId, taskId, sessionId, cols, rows), Context.ConnectionAborted);
    }

    public async Task SendSwarmTaskExecInput(Guid platformId, string taskId, string sessionId, byte[] data)
    {
        await mediator.Send(new SendSwarmTaskExecInput(platformId, taskId, sessionId, data), Context.ConnectionAborted);
    }

    public async Task StartStackExecProcess(Guid stackId, string containerId, string sessionId, string shell)
    {
        await mediator.Send(new StartStackShellSession(stackId, containerId, sessionId, shell), Context.ConnectionAborted);
    }

    public async Task ResizeStackExec(Guid stackId, string containerId, string sessionId, int cols, int rows)
    {
        await mediator.Send(new ResizeStackExecSession(stackId, containerId, sessionId, cols, rows), Context.ConnectionAborted);
    }

    public async Task SendStackExecInput(Guid stackId, string containerId, string sessionId, byte[] data)
    {
        await mediator.Send(new SendStackExecInput(stackId, containerId, sessionId, data), Context.ConnectionAborted);
    }

    #endregion

    private string GetSignalRGroupId(string groupId)
    {
        if (groupId == Constants.WellKnownSignalRGroups.AlertEventsGroup)
            return Constants.WellKnownSignalRGroups.AlertEventsUserGroup(Context.User!.GetUserId());

        return groupId;
    }
}
