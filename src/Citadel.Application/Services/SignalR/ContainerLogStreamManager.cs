using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using System.Buffers;
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
        if (!streams.TryGetValue(groupId, out var context))
            return;

        var containerId = GetNormalizedIdFromGroup(groupId.AsSpan());
        if (string.IsNullOrEmpty(containerId))
        {
            logger.LogError("Invalid group ID format: {GroupId}", groupId);
            return;
        }

        var recentLogs = context.GetBufferedLogsAsBytes();
        if (recentLogs.Length > 0)
        {
            _ = SendBufferedLogsSafe(connectionId, recentLogs);
        }

        OnFirstSubscriber(context, containerId);
    }

    private async Task SendBufferedLogsSafe(string connectionId, byte[] data)
    {
        try
        {
            await dispatcher.SendContainerLogsBatchToConnection(connectionId, data);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to send buffered logs to {Conn}", connectionId);
        }
    }

    private void OnFirstSubscriber(LogStreamContext context, string containerId)
    {
        // Start poll/broadcast if not started
        if (context.TryStart())
        {
            _ = StreamLogsAsync(context, containerId);
        }
        // Start event watcher only once; use dedicated token
        context.EventWatcherTask ??= WatchContainerEvents(context, containerId);
    }

    private async Task StreamLogsAsync(LogStreamContext ctx, string containerId)
    {
        var token = ctx.Cancellation.Token;

        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
            return;

        var channel = ctx.LogChannel;

        try
        {
            var request = new StreamContainerLogsCommand(platform.Address, containerId);
            var connector = connectorFactory.GetConnector(platform.ConnectorType);

            _ = BroadcastBatchesAsync(channel.Reader, containerId, token);

            await foreach (var data in connector.StreamLogsAsync(request, token))
            {
                ctx.AddToBuffer(data.Span);
                ctx.AddToBuffer("\n"u8);
                // Just drop it in the pipe and keep reading
                channel.Writer.TryWrite(data.ToArray());
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerId}", containerId);
        }
        finally
        {
            channel.Writer.TryComplete();
        }
    }

    private async Task BroadcastBatchesAsync(
        ChannelReader<byte[]> reader,
        string containerId,
        CancellationToken token)
    {
        var buffer = new ArrayBufferWriter<byte>(16 * 1024);

        try
        {
            while (await reader.WaitToReadAsync(token))
            {
                await Task.Delay(200, token);

                while (reader.TryRead(out var line))
                {
                    buffer.Write(line);
                    buffer.Write("\n"u8);
                }

                if (buffer.WrittenCount > 0)
                {
                    await dispatcher.SendContainerLogs(containerId, buffer.WrittenSpan.ToArray());
                    buffer.Clear();
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while broadcasting logs for {ContainerId}", containerId);
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
                if (NormalizeDockerId(ev.ContainerId) != containerId)
                    continue;

                if (ev.Action == "start")
                {
                    // Stop current producer/consumer and create fresh ones
                    context.Reset();
                    if (context.TryStart())
                    {
                        _ = StreamLogsAsync(context, containerId);
                    }
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Watcher failed for {ContainerId}", containerId);
        }
        finally
        {
            containerEventBroadcaster.RemoveSubscriber(reader);
        }
    }
}
