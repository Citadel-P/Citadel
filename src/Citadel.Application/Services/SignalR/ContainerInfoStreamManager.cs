using Application.Configs;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.SignalR;

internal sealed class ContainerInfoStreamManager(
    IOptions<JobConfiguration> options,
    IApplicationHubDispatcher dispatcher,
    IServiceScopeFactory scopeFactory,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    ILogger<ContainerInfoStreamManager> logger) : BaseStreamManager<ChannelStreamContext<DockerContainer>>, IStreamGroupManager
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
        if (context.TryStart())
            context.StreamTask = RunStreamAsync(containerId, context);
    }

    private async Task RunStreamAsync(
        string containerId,
        ChannelStreamContext<DockerContainer> context)
    {
        try
        {
            await Task.WhenAll(
                PollDockerStats(containerId, context),
                BroadcastStats(containerId, context));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error in container info streaming for {ContainerId}", containerId);
        }
    }

    private async Task PollDockerStats(string containerId, ChannelStreamContext<DockerContainer> ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = Guid.TryParse(containerId, out var resourceId)
            ? await uow.Containers.GetByIdAsync(resourceId, token)
            : await uow.Containers.GetByIdAsync(containerId, token);
        if (container is null)
        {
            logger.LogError("No Container projection found for container ID {ContainerId}", containerId);
            writer.TryComplete();
            return;
        }
        var platform = await uow.Platforms.GetByIdAsync(container.PlatformId, token);
        if (platform is null)
        {
            writer.TryComplete();
            return;
        }

        try
        {
            var stream = container.DockerNodeId is not null
                ? swarmNodeRuntimeConnector.StreamContainerStatsAsync(
                    platform,
                    container.DockerNodeId,
                    container.DockerContainerId,
                    options.Value.MonitoringInterval * 1000,
                    token)
                : connectorFactory.GetConnector(platform.ConnectorType).StreamContainerStatsAsync(
                    new StreamContainerStatsCommand(
                        container.DockerContainerId,
                        platform.Address,
                        options.Value.MonitoringInterval * 1000),
                    token);
            await foreach (var current in stream)
            {
                await writer.WriteAsync(current, token);
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

    private async Task BroadcastStats(
        string containerReference,
        ChannelStreamContext<DockerContainer> ctx)
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
                        await dispatcher.SendContainerInfo(containerReference, container, token);
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
