using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

internal sealed class ContainerLogStreamManager(
    IDockerHubDispatcher dispatcher,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerLogStreamManager> logger
) : BaseStreamManager<LogStreamContext>, IStreamGroupManager
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

        // Capture buffer before starting live streaming
        var recentLogs = context.GetBufferedLogs();

        // Send buffer only to the newly joined connection
        _ = dispatcher.SendContainerLogsBatchToConnection(connectionId, recentLogs);

        if (context.TryStart())
        {
            Task.Run(async () =>
            {
                try
                {
                    await Task.WhenAll(
                        PollDockerLogs(context, containerId),
                        BroadcastLogs(context, containerId)
                    );
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
            return;
        }

        try
        {
            await foreach (var line in connectorFactory
                .GetConnector(platform.ConnectorType)
                .StreamLogsAsync(new(platform.Address, containerId), token))
            {
                ctx.AddToBuffer(line.Log);
                await writer.WriteAsync(line.Log, token);
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            writer.TryComplete();
        }
    }

    private async Task BroadcastLogs(LogStreamContext ctx, string containerId)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;

        try
        {
            await foreach (var log in reader.ReadAllAsync(token))
            {
                await dispatcher.SendContainerLog(containerId, log);
            }
        }
        catch (OperationCanceledException) { }
    }
}
