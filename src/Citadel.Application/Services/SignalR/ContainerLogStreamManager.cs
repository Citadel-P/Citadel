using System.Text;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

internal sealed class ContainerLogStreamManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ContainerLogStreamManager> logger,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : BaseStreamManager<LogStreamContext>, IStreamGroupManager
{
    protected override void OnSubscriberAdded(string groupId, string connectionId)
    {
        var containerId = GetEntityId(groupId.AsSpan()).ToString();
        if (string.IsNullOrEmpty(containerId))
        {
            logger.LogError("Invalid group ID format: {GroupId}", groupId);
            return;
        }

        if (!streams.TryGetValue(groupId, out var context))
            return;

        var recentLogs = context.GetBufferedLogsAsBytes();
        if (recentLogs.Length > 0)
        {
            _ = dispatcher.SendContainerLogsBatchToConnection(connectionId, recentLogs)
                .ContinueWith(t =>
                {
                    if (t.IsFaulted) logger.LogWarning(t.Exception, "Failed to send buffered logs to {Conn}", connectionId);
                }, TaskContinuationOptions.ExecuteSynchronously);
        }

        OnFirstSubscriber(context, containerId);
    }

    private void OnFirstSubscriber(LogStreamContext context, string containerId)
    {
        // Start poll/broadcast if not started
        if (context.TryStart())
        {
            Run(context, containerId);
        }

        // Start event watcher only once; use dedicated token
        context.EventWatcherTask ??= Task.Run(() => WatchContainerEvents(context, containerId), context.WatcherCts.Token);
    }

    private async Task StreamLogsAsync(LogStreamContext ctx, string containerId)
    {
        var token = ctx.Cancellation.Token;

        if (!platformContainerCache.TryGetPlatformByContainerId(containerId, out var platform))
        {
            logger.LogError("No platform found for container ID {ContainerId}", containerId);
            return;
        }

        try
        {
            await foreach (var data in connectorFactory.GetConnector(platform!.ConnectorType).StreamLogsAsync(new(platform.Address, containerId), token))
            {
                ctx.AddToBuffer(data.Span);
                ctx.AddToBuffer("\n"u8);
                await dispatcher.SendContainerLog(containerId, data);

            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerId}", containerId);
        }
        
    }

    private async Task WatchContainerEvents(LogStreamContext context, string containerId)
    {
        // IMPORTANT: per-context reader to avoid event loss
        var reader = containerEventBroadcaster.AddSubscriber();
        var token = context.WatcherCts.Token;

        try
        {
            await foreach (var ev in reader.ReadAllAsync(token))
            {
                if (ev.ContainerId != containerId)
                    continue;

                if (ev.Action == "start")
                {
                    logger.LogInformation("Container {Id} started, resetting log stream", containerId);

                    // Stop current producer/consumer and create fresh ones
                    context.Reset();

                    if (context.TryStart())
                    {
                        Run(context, containerId);
                    }
                }
                // (optional) we can emit a system message on stop/die if we want
            }
        }
        catch (OperationCanceledException)
        {
            logger.LogInformation("Log streaming watcher for {ContainerId} canceled", containerId);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Watcher failed for {ContainerId}", containerId);
        }
        finally
        {
            containerEventBroadcaster.RemoveSubscriber(reader);
        }
    }

    private void Run(LogStreamContext context, string containerId)
    {
        context.StreamTask = Task.Run(async () =>
        {
            try
            {
                await StreamLogsAsync(context, containerId);
            }
            catch (OperationCanceledException)
            {
                logger.LogInformation("Log streaming for {ContainerId} canceled", containerId);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error in container log streaming for {ContainerId}", containerId);
            }
        }, context.Cancellation.Token);
    }
}