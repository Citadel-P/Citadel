using System.Threading.Channels;
using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services.SignalR;

internal sealed class StackInfoStreamManager(
    IServiceScopeFactory scopeFactory,
    IApplicationHubDispatcher dispatcher,
    IContainerStatsBroadcaster statsBroadcaster,
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
            context.StreamTask = RunStreamAsync(stackId, context);
    }

    private async Task RunStreamAsync(
        Guid stackId,
        ChannelStreamContext<IEnumerable<DockerContainer>> context)
    {
        try
        {
            await Task.WhenAll(
                PollStackContainers(stackId, context),
                BroadcastStats(stackId, context));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error in stack info streaming for {StackId}", stackId);
        }
    }

    private async Task PollStackContainers(Guid stackId, ChannelStreamContext<IEnumerable<DockerContainer>> ctx)
    {
        var writer = ctx.Channel.Writer;
        var token = ctx.Cancellation.Token;
        var latest = new StackContainerStatsAccumulator();
        var statsReader = statsBroadcaster.AddSubscriber();

        try
        {
            var containers = await GetStackContainers(stackId, token);
            latest.Initialize(containers);

            await writer.WriteAsync(latest.Snapshot(), token);

            if (latest.IsEmpty)
                return;

            await foreach (var snapshot in statsReader.ReadAllAsync(token))
            {
                if (latest.Update(snapshot) is { } changedSnapshot)
                    await writer.WriteAsync(changedSnapshot, token);
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while polling stack container stats for {StackId}", stackId);
        }
        finally
        {
            statsBroadcaster.RemoveSubscriber(statsReader);
            writer.TryComplete();
        }
    }

    private async Task<IReadOnlyList<TrackedStackContainer>> GetStackContainers(
        Guid stackId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await unitOfWork.Stacks.GetContainersAsync(stackId, cancellationToken);

        return containers
            .Select(container => new TrackedStackContainer(
                container.Id,
                container.PlatformId,
                new DockerContainer(
                    Name: container.Name,
                    Image: container.Image?.Name ?? string.Empty,
                    Id: container.DockerContainerId,
                    ImageId: container.DockerImageId,
                    State: container.State,
                    Created: container.Created,
                    Stack: container.DockerStack,
                    ContainerStat: null,
                    ControlState: container.ControlState,
                    Ports: container.Ports)))
            .ToList();
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

    private sealed class StackContainerStatsAccumulator
    {
        private readonly Dictionary<Guid, DockerContainer> latest = [];
        private readonly HashSet<Guid> platformIds = [];

        public bool IsEmpty => latest.Count == 0;

        public void Initialize(IEnumerable<TrackedStackContainer> containers)
        {
            foreach (var container in containers)
            {
                latest[container.ContainerId] = container.Container;
                platformIds.Add(container.PlatformId);
            }
        }

        public IReadOnlyList<DockerContainer>? Update(ContainerStatsSnapshot snapshot)
        {
            if (!platformIds.Contains(snapshot.PlatformId))
                return null;

            var changed = false;
            foreach (var stat in snapshot.Stats)
            {
                if (!latest.TryGetValue(stat.ContainerId, out var container))
                    continue;

                latest[stat.ContainerId] = container with
                {
                    ContainerStat = new DockerContainerStat(
                        stat.MemoryActive,
                        stat.MemoryCache,
                        stat.CpuUsage,
                        stat.MemoryLimit,
                        stat.RxBytes,
                        stat.TxBytes,
                        stat.Created)
                };
                changed = true;
            }

            return changed ? Snapshot() : null;
        }

        public IReadOnlyList<DockerContainer> Snapshot() => latest.Values.ToList();
    }

    private sealed record TrackedStackContainer(
        Guid ContainerId,
        Guid PlatformId,
        DockerContainer Container);

    private static bool TryGetStackIdFromGroup(ReadOnlySpan<char> groupId, out Guid stackId)
    {
        stackId = Guid.Empty;
        var idx = groupId.IndexOf(':');

        return (uint)idx < (uint)(groupId.Length - 1) &&
            Guid.TryParse(groupId[(idx + 1)..], out stackId);
    }
}
