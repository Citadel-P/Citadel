using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.Common;
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
        var groupId = Constants.WellKnownSignalRGroups.ContainerLogGroup(normalized);
        TryUseStream(groupId, context =>
        {
            if (!context.TryStartStream(resources => StreamLogsAsync(resources, normalized)))
                return;

            context.EnsureWatcher(token => WatchContainerEvents(context, normalized, token));
        });
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

    private async Task StreamLogsAsync(LogStreamResources resources, string containerId)
    {
        var token = resources.CancellationToken;

        if (!platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
            return;

        var channel = resources.Channel;

        try
        {
            var request = new StreamContainerLogsCommand(platform.Address, containerId);
            var connector = connectorFactory.GetConnector(platform.ConnectorType);

            await LogStreamPipeline.RunAsync(
                channel,
                pipelineToken => ProduceLogsAsync(resources, connector, request, channel.Writer, pipelineToken),
                pipelineToken => BroadcastBatchesAsync(channel.Reader, containerId, pipelineToken),
                token);
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerId}", containerId);
        }
    }

    private static async Task ProduceLogsAsync(
        LogStreamResources resources,
        IContainerConnector connector,
        StreamContainerLogsCommand request,
        ChannelWriter<PooledBuffer> writer,
        CancellationToken token)
    {
        await foreach (var data in connector.StreamLogsAsync(request, token))
        {
            resources.AddToBuffer(data.Span);
            resources.AddToBuffer("\n"u8);

            var pooled = new PooledBuffer(ArrayPool<byte>.Shared.Rent(data.Length), data.Length);
            try
            {
                data.Span.CopyTo(pooled.Buffer);
                await writer.WriteAsync(pooled, token);
            }
            catch
            {
                pooled.Dispose();
                throw;
            }
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
                    using (item)
                    {
                        var len = item.Length;

                        if (len >= BatchSize)
                        {
                            await dispatcher.SendContainerLogs(
                                containerId,
                                item.Buffer.AsMemory(0, len));
                            continue;
                        }

                        if (offset + len + 1 > BatchSize)
                        {
                            await dispatcher.SendContainerLogs(
                                containerId,
                                batch.AsMemory(0, offset));

                            offset = 0;
                        }

                        item.Buffer.AsSpan(0, len).CopyTo(batch.AsSpan(offset));
                        offset += len;
                        batch[offset++] = (byte)'\n';
                    }
                }

                // flush remaining
                if (offset > 0)
                {
                    await dispatcher.SendContainerLogs(containerId, batch.AsMemory(0, offset));
                    offset = 0;
                }
            }
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(batch);
        }
    }

    private async Task WatchContainerEvents(
        LogStreamContext context,
        string containerId,
        CancellationToken token)
    {
        // IMPORTANT: per-context reader to avoid event loss
        var reader = containerEventBroadcaster.AddSubscriber();

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
                    context.TryStartStream(resources => StreamLogsAsync(resources, containerId));
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
