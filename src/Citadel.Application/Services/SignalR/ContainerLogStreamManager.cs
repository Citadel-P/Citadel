using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

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
        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform)) return;

        var currentChannel = ctx.LogChannel;
        try
        {
            var request = new StreamContainerLogsCommand(platform.Address, containerId);
            var connector = connectorFactory.GetConnector(platform.ConnectorType);

            _ = Task.Run(() => BroadcastBatchesAsync(currentChannel, containerId, token), token);

            await foreach (var data in connector.StreamLogsAsync(request, token))
            {
                ctx.AddToBuffer(data.Span);
                ctx.AddToBuffer("\n"u8);

                // Just drop it in the pipe and keep reading
                ctx.LogChannel.Writer.TryWrite(data.ToArray());
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerId}", containerId);
        }
        finally
        {
            currentChannel.Writer.TryComplete();
        }
    }

    private async Task BroadcastBatchesAsync(Channel<byte[]> channel, string containerId, CancellationToken token)
    {
        var reader = channel.Reader;
        var batchBuffer = new MemoryStream();

        try
        {
            while (await reader.WaitToReadAsync(token))
            {
                await Task.Delay(200, token);

                while (reader.TryRead(out var line))
                {
                    batchBuffer.Write(line);
                    batchBuffer.Write("\n"u8);
                }

                if (batchBuffer.Length > 0)
                {
                    await dispatcher.SendContainerLogs(containerId, batchBuffer.ToArray());
                    batchBuffer.SetLength(0);
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while broadcasting logs for {ContainerId}", containerId);
        }
        finally
        {
            await batchBuffer.DisposeAsync();
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
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error in container log streaming for {ContainerId}", containerId);
            }
        }, context.Cancellation.Token);
    }
}