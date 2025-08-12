using System.Text;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

internal sealed class ContainerLogStreamManager(
    IApplicationHubDispatcher dispatcher,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerLogStreamManager> logger) : BaseStreamManager<LogStreamContext>, IStreamGroupManager
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

        // Start producer/consumer only once per group
        if (context.TryStart())
        {
            context.StreamTask = Task.Run(async () =>
            {
                try
                {
                    var poll = PollDockerLogs(context, containerId);
                    var broadcast = BroadcastLogs(context, containerId);
                    await Task.WhenAll(poll, broadcast);
                }
                catch (OperationCanceledException)
                {
                    logger.LogInformation("Log streaming for {ContainerId} canceled", containerId);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error in container log streaming for {ContainerId}", containerId);
                }
            });
        }
    }

    private async Task PollDockerLogs(LogStreamContext ctx, string containerId)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;

        if (!platformContainerCache.TryGetPlatformByContainerId(containerId, out var platform))
        {
            logger.LogError("No platform found for container ID {ContainerId}", containerId);
            writer.TryComplete();
            return;
        }

        try
        {
            await foreach (var data in connectorFactory.GetConnector(platform.ConnectorType).StreamLogsAsync(new(platform.Address, containerId), token).ConfigureAwait(false))
            {
                ctx.AddToBuffer(data.Span);
                ctx.AddToBuffer("\n"u8);
                await writer.WriteAsync(data, token);
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling logs for {ContainerId}", containerId);
        }
        finally
        {
            // ensure writer completes so readers exit cleanly
            writer.TryComplete();
        }
    }

    private async Task BroadcastLogs(LogStreamContext ctx, string containerId)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;

        try
        {
            while (await reader.WaitToReadAsync(token).ConfigureAwait(false))
            {
                while (reader.TryRead(out var log))
                {
                    try
                    {
                        await dispatcher.SendContainerLog(containerId, log).ConfigureAwait(false);
                    }
                    catch (Exception ex)
                    {
                        logger.LogWarning(ex, "Failed to send log to clients for {ContainerId}", containerId);
                    }
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "BroadcastLogs error for {ContainerId}", containerId);
        }
    }
}