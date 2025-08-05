using System.Collections.Concurrent;
using Application.Configs;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using static Hosting.Common.Constants;

namespace Application.Services.SignalR;

internal sealed class ContainerInfoStreamManager(
    IOptions<JobConfiguration> options,
    IContainerInfoHubDispatcher dispatcher, 
    ISignalRConnectionTracker connectionTracker,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerInfoStreamManager> logger) : IContainerInfoStreamManager
{
    private readonly ConcurrentDictionary<string, StreamContext<DockerContainer>> streams = new();

    public void AddSubscriber(string containerId, string connectionId)
    {
        if (!connectionTracker.TryGetUserId(connectionId, out var userId))
        {
            logger.LogWarning("Unknown connection ID {ConnectionId}", connectionId);
            return;
        }

        var group = SignalRGroups.ContainerLogGroup(containerId);
        var context = streams.GetOrAdd(group, _ =>
        {
            var ctx = new StreamContext<DockerContainer>();
            Task.Run(() => PollDockerStats(containerId, ctx));
            Task.Run(() => BroadcastStats(ctx));
            return ctx;
        });

        context.AddSubscriber(connectionId);
        connectionTracker.JoinGroup(group, connectionId, userId);
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

        foreach (var groupId in groups)
        {
            if (!streams.TryGetValue(groupId, out var context))
                continue;

            connectionTracker.LeaveGroup(groupId, connectionId, connectionTracker.TryGetUserId(connectionId, out var userId) ? userId : "unknown");

            CleanUp(context, connectionId, groupId);
        }
    }

    private void CleanUp(StreamContext<DockerContainer> context, string connectionId, string groupId)
    {
        context.RemoveSubscriber(connectionId);
        if (context.IsEmpty)
        {
            context.Cancellation.Cancel();
            streams.TryRemove(groupId, out _);
        }
    }

    private async Task PollDockerStats(string containerId, StreamContext<DockerContainer> ctx)
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
            while (!token.IsCancellationRequested)
            {
                await foreach(var container in connectorFactory.GetConnector(platformInfo.ConnectorType).StreamContainerStatsAsync(new StreamContainerStatsCommand(containerId, platformInfo.Address, options.Value.ContainersInfoInterval * 1000), token))
                {
                    await writer.WriteAsync(container, token);
                }
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            writer.Complete();
        }
    }

    private async Task BroadcastStats(StreamContext<DockerContainer> ctx)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;
        
        try
        {
            await foreach (var container in reader.ReadAllAsync(token))
            {
                using var _ = container;
                await dispatcher.SendContainerInfo(container.Value, token);
            }
        }
        catch (OperationCanceledException) { }
    }

}
