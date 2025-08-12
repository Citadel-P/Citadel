using Application.Configs;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services.SignalR;

internal sealed class ContainerInfoStreamManager(
    IOptions<JobConfiguration> options,
    IApplicationHubDispatcher dispatcher,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerInfoStreamManager> logger) : BaseStreamManager<PooledStreamContext<DockerContainer>>, IStreamGroupManager
{
    protected override void OnSubscriberAdded(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
            return;

        var containerId = GetEntityId(groupId.AsSpan()).ToString();
        if (string.IsNullOrEmpty(containerId))
        {
            logger.LogError("Invalid group ID format: {GroupId}", groupId);
            return;
        }
        if (context.TryStart())
        {
            context.StreamTask = Task.Run(async () =>
            {
                try
                {
                    await Task.WhenAll(
                        PollDockerStats(containerId, context),
                        BroadcastStats(context)
                    );
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error in container info streaming for {ContainerId}", containerId);
                }
            });
        }
    }

    private async Task PollDockerStats(string containerId, PooledStreamContext<DockerContainer> ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;

        if (!platformContainerCache.TryGetPlatformByContainerId(containerId, out var platformInfo))
        {
            logger.LogError("No platform found for container ID {ContainerId}", containerId);
            writer.TryComplete();
            return;
        }

        try
        {
            await foreach (var container in connectorFactory.GetConnector(platformInfo.ConnectorType).StreamContainerStatsAsync(new StreamContainerStatsCommand(containerId, platformInfo.Address, options.Value.ContainersInfoInterval * 1000), token))
            {
                await writer.WriteAsync(container, token);
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling stats for {ContainerId}", containerId);
        }
        finally
        {
            writer.TryComplete();
        }
    }

    private async Task BroadcastStats(PooledStreamContext<DockerContainer> ctx)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;

        try
        {
            while (await reader.WaitToReadAsync(token).ConfigureAwait(false))
            {
                while (reader.TryRead(out var container))
                {
                    try
                    {
                        using (container)
                        {
                            await dispatcher.SendContainerInfo(container.Value, token);
                        }
                    }
                    catch (Exception ex)
                    {
                        logger.LogWarning(ex, "Failed to dispatch container info");
                    }
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "BroadcastStats error");
        }
    }
}