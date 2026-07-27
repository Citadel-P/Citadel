using System.Buffers;
using System.Threading.Channels;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Hosting.Common;
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
    public void StartStackLogs(Guid stackId)
    {
        if (stackId == Guid.Empty)
            return;

        var groupId = Constants.WellKnownSignalRGroups.StackLogGroup(stackId);
        TryUseStream(groupId, context =>
        {
            if (!context.TryStartStream(resources => StreamLogsAsync(resources, stackId)))
                return;

            context.EnsureWatcher(token => WatchContainerEvents(context, stackId, token));
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
                System.Text.Encoding.UTF8.GetBytes($"[{container.Name.TrimStart('/')}] ")))
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
                var transformed = PrefixContainerName(data.Span, container.Prefix);
                if (transformed is null)
                    continue;

                try
                {
                    resources.AddToBuffer(transformed.Span);
                    resources.AddToBuffer("\n"u8);
                    await writer.WriteAsync(transformed, token);
                }
                catch
                {
                    transformed.Dispose();
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

    internal static PooledBuffer? PrefixContainerName(
        ReadOnlySpan<byte> logBytes,
        ReadOnlySpan<byte> prefix)
    {
        while (!logBytes.IsEmpty && logBytes[^1] is (byte)'\r' or (byte)'\n')
            logBytes = logBytes[..^1];

        if (logBytes.IsEmpty)
            return null;

        var outputLength = GetTransformedLength(logBytes, prefix.Length);
        if (outputLength == 0)
            return null;

        var output = new PooledBuffer(ArrayPool<byte>.Shared.Rent(outputLength), outputLength);
        var destination = output.Buffer.AsSpan(0, outputLength);
        var offset = 0;
        var wroteLine = false;

        while (!logBytes.IsEmpty)
        {
            var newlineIndex = logBytes.IndexOf((byte)'\n');
            var line = newlineIndex >= 0 ? logBytes[..newlineIndex] : logBytes;
            logBytes = newlineIndex >= 0 ? logBytes[(newlineIndex + 1)..] : [];
            if (!line.IsEmpty && line[^1] == (byte)'\r')
                line = line[..^1];

            var insertionIndex = GetPrefixInsertionIndex(line);
            if (insertionIndex < 0)
                continue;

            if (wroteLine)
                destination[offset++] = (byte)'\n';

            line[..insertionIndex].CopyTo(destination[offset..]);
            offset += insertionIndex;
            prefix.CopyTo(destination[offset..]);
            offset += prefix.Length;
            line[insertionIndex..].CopyTo(destination[offset..]);
            offset += line.Length - insertionIndex;
            wroteLine = true;
        }

        return output;
    }

    private static int GetTransformedLength(ReadOnlySpan<byte> logBytes, int prefixLength)
    {
        var length = 0;
        var lineCount = 0;

        while (!logBytes.IsEmpty)
        {
            var newlineIndex = logBytes.IndexOf((byte)'\n');
            var line = newlineIndex >= 0 ? logBytes[..newlineIndex] : logBytes;
            logBytes = newlineIndex >= 0 ? logBytes[(newlineIndex + 1)..] : [];
            if (!line.IsEmpty && line[^1] == (byte)'\r')
                line = line[..^1];

            if (GetPrefixInsertionIndex(line) < 0)
                continue;

            length += line.Length + prefixLength;
            if (lineCount++ > 0)
                length++;
        }

        return length;
    }

    private static int GetPrefixInsertionIndex(ReadOnlySpan<byte> line)
    {
        if (IsDockerTimestampOnly(line))
            return -1;

        var firstSpaceIndex = line.IndexOf((byte)' ');
        if (firstSpaceIndex <= 0 || !IsDockerTimestampOnly(line[..firstSpaceIndex]))
            return 0;

        if (IsAsciiWhitespace(line[(firstSpaceIndex + 1)..]))
        {
            return -1;
        }

        return firstSpaceIndex + 1;
    }

    private static bool IsDockerTimestampOnly(ReadOnlySpan<byte> value)
    {
        var span = TrimAsciiWhitespace(value);
        return span.Length >= 20 &&
            span[4] == (byte)'-' &&
            span[7] == (byte)'-' &&
            span[10] == (byte)'T' &&
            span[13] == (byte)':' &&
            span[16] == (byte)':' &&
            span[^1] == (byte)'Z';
    }

    private static bool IsAsciiWhitespace(ReadOnlySpan<byte> value)
    {
        foreach (var character in value)
        {
            if (character is not ((byte)' ' or (byte)'\t' or (byte)'\r' or (byte)'\n'))
                return false;
        }

        return true;
    }

    private static ReadOnlySpan<byte> TrimAsciiWhitespace(ReadOnlySpan<byte> value)
    {
        while (!value.IsEmpty && value[0] is (byte)' ' or (byte)'\t' or (byte)'\r' or (byte)'\n')
            value = value[1..];
        while (!value.IsEmpty && value[^1] is (byte)' ' or (byte)'\t' or (byte)'\r' or (byte)'\n')
            value = value[..^1];
        return value;
    }

    private sealed record StackLogContainer(string Id, byte[] Prefix);
}
