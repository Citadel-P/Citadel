using System.Collections.Concurrent;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;
using static Hosting.Common.Constants;

namespace Application.Services.SignalR;

internal sealed class ContainerLogStreamManager(
    IContainerLogHubDispatcher dispatcher,
    ISignalRConnectionTracker connectionTracker,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerLogStreamManager> logger) : IContainerLogStreamManager
{
    private readonly ConcurrentDictionary<string, LogStreamContext> streams = new(); // groupId -> LogStreamContext

    public async Task AddSubscriber(string containerId, string connectionId)
    {
        if (!connectionTracker.TryGetUserId(connectionId, out var userId))
        {
            logger.LogWarning("Unknown connection ID {ConnectionId}", connectionId);
            return;
        }

        var group = SignalRGroups.ContainerLogGroup(containerId);
        // Ensure if a previous context is completed (due to cancellation), we never reuse it
        if (streams.TryGetValue(group, out var existing) && existing.Channel.Reader.Completion.IsCompleted)
        {
            streams.TryRemove(group, out _);
        }

        var context = streams.GetOrAdd(group, _ => new LogStreamContext(containerId));

        var recentLogs = context.GetBufferedLogs();
        await dispatcher.SendContainerLogsBatch(containerId, recentLogs);

        context.AddSubscriber(connectionId);
        connectionTracker.JoinGroup(group, connectionId, userId);

        // Start streaming logs only if not already started
        if (context.Started.TrySet())
        {
            _ = Task.Run(() => PollDockerLogs(containerId, context));
            _ = Task.Run(() => BroadcastLogs(context));
        }
    }

    public void RemoveSubscriber(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
            return;

        CleanUp(context, connectionId, groupId);
    }

    public void RemoveConnection(string connectionId)
    {
        var groups = connectionTracker.GetGroupsForConnection(connectionId);

        foreach (var group in groups)
        {
            if (!streams.TryGetValue(group, out var context))
                continue;

            connectionTracker.LeaveGroup(group, connectionId, connectionTracker.TryGetUserId(connectionId, out var userId) ? userId : "unknown");

            CleanUp(context, connectionId, group);
        }
    }

    private void CleanUp(LogStreamContext context, string connectionId, string groupId)
    {
        context.RemoveSubscriber(connectionId);
        if (context.IsEmpty)
        {
            context.Cancellation.Cancel();
            context.Clear();
            streams.TryRemove(groupId, out _);
        }
    }

    private async Task PollDockerLogs(string containerId, LogStreamContext ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;

        if (!platformContainerCache.TryGetPlatformByContainerId(containerId, out var platformInfo))
        {
            logger.LogError("No platform found for container ID {ContainerId}", containerId);
            return;
        }

        try
        {
            await foreach (var line in connectorFactory
                .GetConnector(platformInfo.ConnectorType)
                .StreamLogsAsync(new(platformInfo.Address, containerId), token))
            {
                ctx.AddToBuffer(line.Log);
                await writer.WriteAsync(line.Log, token);
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            writer.Complete();
        }
    }

    private async Task BroadcastLogs(LogStreamContext ctx)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;

        try
        {
            await foreach (var log in reader.ReadAllAsync(token))
            {
                await dispatcher.SendContainerLog(ctx.ContainerId, log);
            }
        }
        catch (OperationCanceledException) { }
    }
}
