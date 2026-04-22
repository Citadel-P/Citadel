using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using System.Buffers;
using System.Threading.Channels;

namespace Application.Services.SignalR;

internal interface IContainerLogStreamManager : IStreamGroupManager
{
    void StartContainerLogs(string containerId);
}

internal sealed class ContainerLogStreamManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ContainerLogStreamManager> logger,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : BaseStreamManager<LogStreamContext>, IContainerLogStreamManager
{
    public void StartContainerLogs(string containerId)
    {
        if (string.IsNullOrWhiteSpace(containerId))
            return;

        var normalized = NormalizeDockerId(containerId);
        var groupId = $"container-log:{normalized}";

        var context = streams.GetOrAdd(groupId, _ => new LogStreamContext());

        // Start stream orchestration explicitly. Do not treat Start as a subscriber
        // registration. Start should only start the producer/consumer and watcher.
        if (!context.TryStart())
            return;

        context.StreamTask = StreamLogsAsync(context, normalized);
        context.EventWatcherTask ??= WatchContainerEvents(context, normalized);
    }

    protected override void OnSubscriberAdded(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
            return;

        var recentLogs = context.GetBufferedLogsAsBytes();
        if (recentLogs.Length > 0)
        {
            _ = SendBufferedLogsSafe(connectionId, recentLogs);
        }
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

                var rented = ArrayPool<byte>.Shared.Rent(data.Length);

                try
                {
                    data.Span.CopyTo(rented);
                    var pooled = new PooledBuffer(rented, data.Length);

                    // backpressure
                    await channel.Writer.WriteAsync(pooled, token);
                }
                catch
                {
                    ArrayPool<byte>.Shared.Return(rented);
                    throw;
                }
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
    ChannelReader<PooledBuffer> reader,
    string containerId,
    CancellationToken token)
    {
        const int BatchSize = 16 * 1024;
        byte[] batch = ArrayPool<byte>.Shared.Rent(BatchSize);
        int offset = 0;

        try
        {
            while (await reader.WaitToReadAsync(token))
            {
                while (reader.TryRead(out var item))
                {
                    var len = item.Length;

                    if (len > BatchSize)
                    {
                        await dispatcher.SendContainerLogs(
                            containerId,
                            item.Buffer.AsMemory(0, len));

                        item.Dispose();
                        continue;
                    }

                    if (offset + len > BatchSize)
                    {
                        await dispatcher.SendContainerLogs(
                            containerId,
                            batch.AsMemory(0, offset));

                        offset = 0;
                    }

                    item.Buffer.AsSpan(0, len).CopyTo(batch.AsSpan(offset));
                    offset += len;

                    batch[offset++] = (byte)'\n';

                    item.Dispose();
                }

                // flush remaining
                if (offset > 0)
                {
                    await dispatcher.SendContainerLogs(containerId, batch.AsMemory(0, offset));
                    offset = 0;
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
            ArrayPool<byte>.Shared.Return(batch);
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
