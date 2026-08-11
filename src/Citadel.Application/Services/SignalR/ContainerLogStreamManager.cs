using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.Common;
using Microsoft.Extensions.Logging;
using System.Buffers;
using System.Threading.Channels;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.SignalR;

internal interface IContainerLogStreamManager : IStreamGroupManager
{
    void StartContainerLogs(string containerId);
}

internal sealed class ContainerLogStreamManager(
    IApplicationHubDispatcher dispatcher,
    ILogger<ContainerLogStreamManager> logger,
    IServiceScopeFactory scopeFactory,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector)
    : BaseStreamManager<LogStreamContext>, IContainerLogStreamManager
{
    public void StartContainerLogs(string containerId)
    {
        if (string.IsNullOrWhiteSpace(containerId))
            return;

        var containerReference = NormalizeContainerReference(containerId);
        var groupId = Constants.WellKnownSignalRGroups.ContainerLogGroup(containerReference);
        TryUseStream(groupId, context =>
        {
            if (!context.TryStartStream(resources => StreamLogsAsync(resources, containerReference)))
                return;

            context.EnsureWatcher(token => WatchContainerEvents(context, containerReference, token));
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

    private async Task StreamLogsAsync(LogStreamResources resources, string containerReference)
    {
        var token = resources.CancellationToken;

        var target = await ResolveTargetAsync(containerReference, token);
        if (target is null)
            return;

        var channel = resources.Channel;

        try
        {
            var stream = target.DockerNodeId is not null
                ? swarmNodeRuntimeConnector.StreamContainerLogsAsync(
                    target.Platform,
                    target.DockerNodeId,
                    target.DockerContainerId,
                    token)
                : connectorFactory.GetConnector(target.Platform.ConnectorType).StreamLogsAsync(
                    new StreamContainerLogsCommand(target.Platform.Address, target.DockerContainerId),
                    token);

            await LogStreamPipeline.RunAsync(
                channel,
                pipelineToken => ProduceLogsAsync(resources, stream, channel.Writer, pipelineToken),
                pipelineToken => BroadcastBatchesAsync(channel.Reader, containerReference, pipelineToken),
                token);
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerReference}", containerReference);
        }
    }

    private static async Task ProduceLogsAsync(
        LogStreamResources resources,
        IAsyncEnumerable<ReadOnlyMemory<byte>> stream,
        ChannelWriter<PooledBuffer> writer,
        CancellationToken token)
    {
        await foreach (var data in stream.WithCancellation(token))
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
        string containerReference,
        CancellationToken token)
    {
        var target = await ResolveTargetAsync(containerReference, token);
        if (target is null)
            return;

        // IMPORTANT: per-context reader to avoid event loss
        var reader = containerEventBroadcaster.AddSubscriber();

        try
        {
            await foreach (var ev in reader.ReadAllAsync(token))
            {
                if (ev.PlatformId != target.Platform.Id
                    || !string.Equals(ev.DockerNodeId, target.DockerNodeId, StringComparison.Ordinal)
                    || !string.Equals(ev.ContainerId, target.DockerContainerId, StringComparison.OrdinalIgnoreCase))
                    continue;

                if (ev.Action == "start")
                {
                    // Stop current producer/consumer and create fresh ones
                    context.Reset();
                    context.TryStartStream(resources => StreamLogsAsync(resources, containerReference));
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Watcher failed for {ContainerReference}", containerReference);
        }
        finally
        {
            containerEventBroadcaster.RemoveSubscriber(reader);
        }
    }

    private async Task<ContainerLogTarget?> ResolveTargetAsync(
        string containerReference,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = Guid.TryParse(containerReference, out var resourceId)
            ? await uow.Containers.GetByIdAsync(resourceId, cancellationToken)
            : await uow.Containers.GetByIdAsync(containerReference, cancellationToken);
        if (container is null)
            return null;

        var platform = await uow.Platforms.GetByIdAsync(container.PlatformId, cancellationToken);
        return platform is null
            ? null
            : new ContainerLogTarget(platform, container.DockerNodeId, container.DockerContainerId);
    }

    private sealed record ContainerLogTarget(
        Domain.Entities.Platforms.Platform Platform,
        string? DockerNodeId,
        string DockerContainerId);
}
