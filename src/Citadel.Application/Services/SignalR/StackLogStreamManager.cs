using System.Buffers;
using System.Text;
using System.Threading.Channels;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

internal interface IStackLogStreamManager : IStreamGroupManager
{
    void StartStackLogs(Guid stackId);
}

internal sealed class StackLogStreamManager(
    IServiceScopeFactory scopeFactory,
    IApplicationHubDispatcher dispatcher,
    ILogger<StackLogStreamManager> logger,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : BaseStreamManager<LogStreamContext>, IStackLogStreamManager
{
    private static readonly UTF8Encoding Utf8NoBom = new(false);

    public void StartStackLogs(Guid stackId)
    {
        if (stackId == Guid.Empty)
            return;

        var groupId = $"stack-log:{stackId}";
        var context = streams.GetOrAdd(groupId, _ => new LogStreamContext());

        if (!context.TryStartStream(resources => StreamLogsAsync(resources, stackId)))
            return;

        context.EnsureWatcher(token => WatchContainerEvents(context, stackId, token));
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
            await dispatcher.SendStackLogsBatchToConnection(connectionId, data);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to send buffered stack logs to {Conn}", connectionId);
        }
    }

    private async Task StreamLogsAsync(LogStreamResources resources, Guid stackId)
    {
        var token = resources.CancellationToken;
        var channel = resources.Channel;

        try
        {
            var containers = await GetStackLogContainers(stackId, token);
            if (containers.Count == 0)
                return;

            await LogStreamPipeline.RunAsync(
                channel,
                pipelineToken => Task.WhenAll(
                    containers.Select(container => StreamContainerLogsAsync(
                        resources,
                        channel.Writer,
                        container,
                        pipelineToken))),
                pipelineToken => BroadcastBatchesAsync(channel.Reader, stackId, pipelineToken),
                token);
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for stack {StackId}", stackId);
        }
    }

    private async Task<IReadOnlyList<StackLogContainer>> GetStackLogContainers(Guid stackId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await unitOfWork.Stacks.GetContainersAsync(stackId, cancellationToken);

        return containers
            .Where(container => !string.IsNullOrWhiteSpace(container.DockerContainerId))
            .Select(container => new StackLogContainer(
                NormalizeDockerId(container.DockerContainerId),
                container.Name.TrimStart('/')))
            .ToList();
    }

    private async Task StreamContainerLogsAsync(
        LogStreamResources resources,
        ChannelWriter<PooledBuffer> writer,
        StackLogContainer container,
        CancellationToken token)
    {
        if (!platformContainerCache.TryGetPlatformWithContainer(container.Id, out var platform))
        {
            logger.LogWarning("No platform found for stack container {ContainerId}", container.Id);
            return;
        }

        try
        {
            var request = new StreamContainerLogsCommand(platform.Address, container.Id);
            var connector = connectorFactory.GetConnector(platform.ConnectorType);

            await foreach (var data in connector.StreamLogsAsync(request, token))
            {
                var transformed = PrefixContainerName(data.Span, container.Name);
                if (transformed.Length == 0)
                    continue;

                resources.AddToBuffer(transformed);
                resources.AddToBuffer("\n"u8);

                var pooled = new PooledBuffer(ArrayPool<byte>.Shared.Rent(transformed.Length), transformed.Length);
                try
                {
                    transformed.CopyTo(pooled.Buffer);
                    await writer.WriteAsync(pooled, token);
                }
                catch
                {
                    pooled.Dispose();
                    throw;
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for stack container {ContainerId}", container.Id);
        }
    }

    private async Task BroadcastBatchesAsync(ChannelReader<PooledBuffer> reader, Guid stackId, CancellationToken token)
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
                            await dispatcher.SendStackLogs(stackId, item.Buffer.AsMemory(0, len));
                            continue;
                        }

                        if (offset + len + 1 > BatchSize)
                        {
                            await dispatcher.SendStackLogs(stackId, batch.AsMemory(0, offset));
                            offset = 0;
                        }

                        item.Buffer.AsSpan(0, len).CopyTo(batch.AsSpan(offset));
                        offset += len;
                        batch[offset++] = (byte)'\n';
                    }
                }

                if (offset > 0)
                {
                    await dispatcher.SendStackLogs(stackId, batch.AsMemory(0, offset));
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
        Guid stackId,
        CancellationToken token)
    {
        var reader = containerEventBroadcaster.AddSubscriber();

        try
        {
            await foreach (var ev in reader.ReadAllAsync(token))
            {
                if (ev.Action != "start")
                    continue;

                var containerIds = await GetStackContainerIds(stackId, token);
                if (!containerIds.Contains(NormalizeDockerId(ev.ContainerId)))
                    continue;

                context.Reset();
                context.TryStartStream(resources => StreamLogsAsync(resources, stackId));
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Stack log watcher failed for {StackId}", stackId);
        }
        finally
        {
            containerEventBroadcaster.RemoveSubscriber(reader);
        }
    }

    private async Task<HashSet<string>> GetStackContainerIds(Guid stackId, CancellationToken cancellationToken)
    {
        var containers = await GetStackLogContainers(stackId, cancellationToken);
        return containers.Select(container => container.Id).ToHashSet(StringComparer.OrdinalIgnoreCase);
    }

    private static byte[] PrefixContainerName(ReadOnlySpan<byte> logBytes, string containerName)
    {
        var text = Utf8NoBom.GetString(logBytes).TrimEnd('\r', '\n');
        if (string.IsNullOrEmpty(text))
            return [];

        var prefix = $"[{containerName}] ";
        var lines = text.Split('\n');
        for (var i = 0; i < lines.Length; i++)
        {
            var line = lines[i].TrimEnd('\r');
            if (IsDockerTimestampOnly(line))
            {
                lines[i] = string.Empty;
                continue;
            }

            var firstSpaceIndex = line.IndexOf(' ');
            if (firstSpaceIndex > 0 && IsDockerTimestampOnly(line[..firstSpaceIndex]) && string.IsNullOrWhiteSpace(line[(firstSpaceIndex + 1)..]))
            {
                lines[i] = string.Empty;
                continue;
            }

            lines[i] = firstSpaceIndex > 0
                ? string.Concat(line.AsSpan(0, firstSpaceIndex + 1), prefix, line.AsSpan(firstSpaceIndex + 1))
                : prefix + line;
        }

        var prefixed = string.Join('\n', lines.Where(line => !string.IsNullOrEmpty(line)));
        return string.IsNullOrEmpty(prefixed) ? [] : Utf8NoBom.GetBytes(prefixed);
    }

    private static bool IsDockerTimestampOnly(string value)
    {
        var span = value.AsSpan().Trim();
        return span.Length >= 20 &&
            span[4] == '-' &&
            span[7] == '-' &&
            span[10] == 'T' &&
            span[13] == ':' &&
            span[16] == ':' &&
            span[^1] == 'Z';
    }

    private sealed record StackLogContainer(string Id, string Name);
}
