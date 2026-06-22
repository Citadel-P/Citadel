using System.Threading.Channels;
using Application.Configs;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services.SignalR;

internal sealed class StackInfoStreamManager(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    IApplicationHubDispatcher dispatcher,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<StackInfoStreamManager> logger) : BaseStreamManager<ChannelStreamContext<IEnumerable<DockerContainer>>>, IStreamGroupManager
{
    protected override void OnSubscriberAdded(string groupId, string connectionId)
    {
        if (!streams.TryGetValue(groupId, out var context))
            return;

        if (!TryGetStackIdFromGroup(groupId.AsSpan(), out var stackId))
        {
            logger.LogError("Invalid stack info group ID format: {GroupId}", groupId);
            return;
        }

        if (context.TryStart())
        {
            context.StreamTask = Task.Run(async () =>
            {
                try
                {
                    await Task.WhenAll(
                        PollStackContainers(stackId, context),
                        BroadcastStats(stackId, context)
                    );
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error in stack info streaming for {StackId}", stackId);
                }
            });
        }
    }

    private async Task PollStackContainers(Guid stackId, ChannelStreamContext<IEnumerable<DockerContainer>> ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;
        var latest = new Dictionary<string, DockerContainer>(StringComparer.OrdinalIgnoreCase);
        var gate = new SemaphoreSlim(1, 1);

        try
        {
            var containers = await GetStackContainers(stackId, token);
            foreach (var container in containers)
            {
                latest[NormalizeDockerId(container.Id)] = container;
            }

            await writer.WriteAsync(latest.Values.ToList(), token);

            var containerIds = latest.Keys.ToArray();
            if (containerIds.Length == 0)
                return;

            if (!platformContainerCache.TryGetPlatformsWithContainers(containerIds, out var platformEntries))
            {
                logger.LogWarning("No online platform cache entries found for stack {StackId}", stackId);
                return;
            }

            var pollTasks = platformEntries
                .SelectMany(entry => entry.Containers.Keys.Select(containerId =>
                    PollContainerStats(containerId, entry, latest, gate, writer, token)));

            await Task.WhenAll(pollTasks);
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling stack container stats for {StackId}", stackId);
        }
        finally
        {
            writer.TryComplete();
            gate.Dispose();
        }
    }

    private async Task<IReadOnlyList<DockerContainer>> GetStackContainers(Guid stackId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await unitOfWork.Stacks.GetContainersAsync(stackId, cancellationToken);

        return containers
            .Select(container => new DockerContainer(
                Name: container.Name,
                Image: container.Image?.Name ?? string.Empty,
                Id: container.DockerContainerId,
                ImageId: container.DockerImageId,
                State: container.State,
                Created: container.Created,
                Stack: container.DockerStack,
                ContainerStat: null,
                ControlState: container.ControlState,
                Ports: container.Ports))
            .ToList();
    }

    private async Task PollContainerStats(
        string containerId,
        PlatformCacheEntry platformInfo,
        Dictionary<string, DockerContainer> latest,
        SemaphoreSlim gate,
        ChannelWriter<IEnumerable<DockerContainer>> writer,
        CancellationToken token)
    {
        try
        {
            await foreach (var container in connectorFactory
                .GetConnector(platformInfo.ConnectorType)
                .StreamContainerStatsAsync(
                    new StreamContainerStatsCommand(containerId, platformInfo.Address, options.Value.MonitoringInterval * 1000),
                    token))
            {
                var normalizedId = NormalizeDockerId(string.IsNullOrWhiteSpace(container.Id) ? containerId : container.Id);

                await gate.WaitAsync(token);
                try
                {
                    latest[normalizedId] = latest.TryGetValue(normalizedId, out var previous)
                        ? MergeContainer(previous, container)
                        : container;

                    await writer.WriteAsync(latest.Values.ToList(), token);
                }
                finally
                {
                    gate.Release();
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling stats for stack container {ContainerId}", containerId);
        }
    }

    private async Task BroadcastStats(Guid stackId, ChannelStreamContext<IEnumerable<DockerContainer>> ctx)
    {
        var reader = ctx.Channel.Reader;
        var token = ctx.Cancellation.Token;

        try
        {
            while (await reader.WaitToReadAsync(token).ConfigureAwait(false))
            {
                while (reader.TryRead(out var containers))
                {
                    try
                    {
                        await dispatcher.SendStackContainersInfo(stackId, containers, token);
                    }
                    catch (Exception ex)
                    {
                        logger.LogWarning(ex, "Failed to dispatch stack container info");
                    }
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Broadcast stack stats error");
        }
    }

    private static DockerContainer MergeContainer(DockerContainer previous, DockerContainer current) =>
        current with
        {
            Name = string.IsNullOrWhiteSpace(current.Name) ? previous.Name : current.Name,
            Image = string.IsNullOrWhiteSpace(current.Image) ? previous.Image : current.Image,
            Id = string.IsNullOrWhiteSpace(current.Id) ? previous.Id : current.Id,
            ImageId = string.IsNullOrWhiteSpace(current.ImageId) ? previous.ImageId : current.ImageId,
            Created = current.Created ?? previous.Created,
            Stack = current.Stack ?? previous.Stack,
            ContainerStat = current.ContainerStat ?? previous.ContainerStat,
            ControlState = current.ControlState ?? previous.ControlState,
            Ports = current.Ports ?? previous.Ports
        };

    private static bool TryGetStackIdFromGroup(ReadOnlySpan<char> groupId, out Guid stackId)
    {
        stackId = Guid.Empty;
        var idx = groupId.IndexOf(':');

        return (uint)idx < (uint)(groupId.Length - 1) &&
            Guid.TryParse(groupId[(idx + 1)..], out stackId);
    }
}