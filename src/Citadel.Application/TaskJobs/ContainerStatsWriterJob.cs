using Application.Configs;
using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class ContainerStatsWriterJob(
    IDbWorkQueue dbQueue,
    IOptions<JobConfiguration> options,
    INotificationQueue notificationQueue,
    ChannelReader<ContainersStatBatch> reader,
    IPlatformContainerCache platformContainerCache,
    IContainerStatsBroadcaster statsBroadcaster,
    IContainerStreamManager containersStreamManager,
    ILogger<ContainerStatsWriterJob> logger) : BackgroundService
{
    private readonly JobConfiguration _config = options.Value;
    private Dictionary<Guid, List<ContainerStat>> _buffer = [];
    private Dictionary<Guid, SwarmNodeStatsSource> _nodeScopedContainers = [];
    private int _bufferedCount;
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        using var flushTimer = new PeriodicTimer(
            TimeSpan.FromSeconds(Math.Max(1, _config.FlashInterval)));

        try
        {
            var readTask = reader.WaitToReadAsync(cancellationToken).AsTask();
            var flushTask = flushTimer.WaitForNextTickAsync(cancellationToken).AsTask();

            while (!cancellationToken.IsCancellationRequested)
            {
                var completed = await Task.WhenAny(readTask, flushTask);
                if (completed == flushTask)
                {
                    if (!await flushTask)
                        break;

                    if (_bufferedCount > 0)
                        await FlushAsync(cancellationToken);

                    flushTask = flushTimer.WaitForNextTickAsync(cancellationToken).AsTask();
                    continue;
                }

                if (!await readTask)
                    break;

                while (reader.TryRead(out var batch))
                {
                    try
                    {
                        Accumulate(batch);
                        await DispatchLiveStatsAsync(batch, cancellationToken);
                    }
                    finally
                    {
                        batch.Release();
                    }

                    if (ShouldFlush())
                        await FlushAsync(cancellationToken);
                }

                readTask = reader.WaitToReadAsync(cancellationToken).AsTask();
            }
        }
        catch (OperationCanceledException) { }
        finally
        {
            while (reader.TryRead(out var batch))
                batch.Release();
        }

        if (!cancellationToken.IsCancellationRequested && _bufferedCount > 0)
            await FlushAsync(cancellationToken);
    }

    private async ValueTask DispatchLiveStatsAsync(
        ContainersStatBatch batch,
        CancellationToken cancellationToken)
    {
        var hasStatsSubscribers = statsBroadcaster.HasSubscribers;
        var hasNotificationSubscribers = containersStreamManager.HasStatsSubscribers(batch.PlatformId);
        if (!hasStatsSubscribers && !hasNotificationSubscribers)
            return;

        // Both consumers can safely share this immutable snapshot.
        var snapshot = batch.Stats.ToArray();

        if (hasNotificationSubscribers)
        {
            try
            {
                await notificationQueue.EnqueueAsync(
                    new SendContainersNotificationWorkItem(
                        containersStreamManager,
                        batch.PlatformId,
                        snapshot),
                    cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogWarning(
                    ex,
                    "Failed to enqueue container stats notification for platform {PlatformId}.",
                    batch.PlatformId);
            }
        }

        if (hasStatsSubscribers)
            await statsBroadcaster.PublishAsync(
                new ContainerStatsSnapshot(batch.PlatformId, snapshot),
                cancellationToken);
    }

    private void Accumulate(ContainersStatBatch batch)
    {
        if (!_buffer.TryGetValue(batch.PlatformId, out var list))
        {
            list = [];
            _buffer[batch.PlatformId] = list;
        }

        list.AddRange(batch.Stats);
        if (!string.IsNullOrWhiteSpace(batch.DockerNodeId))
        {
            var source = new SwarmNodeStatsSource(
                batch.PlatformId,
                batch.DockerNodeId,
                DateTimeOffset.UtcNow);
            foreach (var stat in batch.Stats)
                _nodeScopedContainers[stat.ContainerId] = source;
        }
        _bufferedCount += batch.Stats.Count;
    }

    private bool ShouldFlush() =>
        _bufferedCount >= _config.BatchSize ||
        (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(_config.FlashInterval);

    private async Task FlushAsync(CancellationToken ct)
    {
        if (_bufferedCount == 0) return;

        var dataToFlush = _buffer;
        var nodeScopedContainers = _nodeScopedContainers;
        var retryDelay = TimeSpan.FromSeconds(1);

        while (true)
        {
            try
            {
                await dbQueue.EnqueueAndWaitAsync(
                    new ContainerStatsBatchWorkItem(dataToFlush, nodeScopedContainers, platformContainerCache, logger),
                    ct);
                _buffer = [];
                _nodeScopedContainers = [];
                _bufferedCount = 0;
                _lastFlush = DateTime.UtcNow;
                return;
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Failed to persist container stats batch. Retrying in {Delay}.",
                    retryDelay);
                await Task.Delay(retryDelay, ct);
                retryDelay = TimeSpan.FromSeconds(Math.Min(30, retryDelay.TotalSeconds * 2));
            }
        }
    }
}

internal sealed class ContainerStatsBatchWorkItem(
    IReadOnlyDictionary<Guid, List<ContainerStat>> batch,
    IReadOnlyDictionary<Guid, SwarmNodeStatsSource> nodeScopedContainers,
    IPlatformContainerCache platformContainerCache,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        // Flatten the dictionary into a single list for bulk insert
        var filteredList = new List<ContainerStat>();
        var droppedCount = 0;

        foreach (var (platformId, stats) in batch)
        {
            if (!platformContainerCache.TryGetContainers(platformId, out var containers))
            {
                droppedCount += stats.Count;
                continue;
            }

            var containerIds = new HashSet<Guid>();
            foreach (var containerId in containers.Values)
            {
                containerIds.Add(containerId);
            }

            foreach (var stat in stats)
            {
                if (containerIds.Contains(stat.ContainerId))
                    filteredList.Add(stat);
                else
                    droppedCount++;
            }
        }

        if (nodeScopedContainers.Count > 0 && filteredList.Count > 0)
        {
            var scopedContainerIds = filteredList
                .Where(stat => nodeScopedContainers.ContainsKey(stat.ContainerId))
                .Select(static stat => stat.ContainerId)
                .Distinct()
                .ToArray();
            if (scopedContainerIds.Length > 0)
            {
                var persistedContainers = (await uow.Containers.GetByIdAsync(scopedContainerIds, token))
                    .ToDictionary(static container => container.Id);
                var removed = filteredList.RemoveAll(stat =>
                {
                    if (!nodeScopedContainers.TryGetValue(stat.ContainerId, out var source))
                        return false;
                    return !persistedContainers.TryGetValue(stat.ContainerId, out var container)
                           || container.PlatformId != source.PlatformId
                           || !string.Equals(container.DockerNodeId, source.DockerNodeId, StringComparison.Ordinal);
                });
                droppedCount += removed;
            }
        }

        if (droppedCount > 0)
        {
            logger.LogWarning(
                "Dropped {Count} container stats because their persisted Platform or Node ownership no longer matched when the batch was flushed.",
                droppedCount);
        }

        if (filteredList.Count == 0) return;

        var swarmServiceStats = await BuildSwarmServiceStatsAsync(
            uow,
            filteredList,
            nodeScopedContainers,
            token);
        await uow.ContainerStats.BulkInsertAsync(filteredList, token);
        if (swarmServiceStats.Count > 0)
            await uow.SwarmServiceStats.BulkInsertAsync(swarmServiceStats, token);
        var nodeSamples = filteredList
            .Where(stat => nodeScopedContainers.ContainsKey(stat.ContainerId))
            .Select(stat => nodeScopedContainers[stat.ContainerId])
            .GroupBy(static source => (source.PlatformId, source.DockerNodeId))
            .Select(static group => group.MaxBy(static source => source.ObservedAt)!)
            .ToArray();
        foreach (var source in nodeSamples)
        {
            var state = await uow.Swarm.GetNodeRuntimeStateAsync(
                source.PlatformId,
                source.DockerNodeId,
                token);
            if (state is not null)
            {
                await uow.Swarm.UpsertNodeRuntimeStateAsync(state with
                {
                    LastStatsSampleAt = source.ObservedAt
                }, token);
            }
        }
        await uow.CommitAsync(token);
    }

    private static async Task<IReadOnlyList<SwarmServiceStat>> BuildSwarmServiceStatsAsync(
        IUnitOfWork uow,
        IReadOnlyList<ContainerStat> stats,
        IReadOnlyDictionary<Guid, SwarmNodeStatsSource> nodeScopedContainers,
        CancellationToken cancellationToken)
    {
        var candidateContainerIds = stats
            .Where(stat => nodeScopedContainers.ContainsKey(stat.ContainerId))
            .Select(static stat => stat.ContainerId)
            .Distinct()
            .ToArray();
        if (candidateContainerIds.Length == 0)
            return [];

        var attributions = await uow.SwarmServiceStats.GetAttributionsAsync(
            candidateContainerIds,
            cancellationToken);
        if (attributions.Count == 0)
            return [];

        var byContainer = attributions.ToDictionary(static value => value.ContainerId);
        var result = new List<SwarmServiceStat>(stats.Count);
        foreach (var stat in stats)
        {
            if (stat.Created is not { } created
                || !byContainer.TryGetValue(stat.ContainerId, out var attribution))
            {
                continue;
            }

            result.Add(new SwarmServiceStat(
                attribution.PlatformId,
                attribution.DockerServiceId,
                attribution.SwarmServiceId,
                attribution.StackId,
                attribution.ServiceName,
                attribution.TaskKey,
                attribution.DockerTaskId,
                stat.MemoryActive,
                stat.MemoryCache,
                stat.CpuUsage,
                stat.MemoryLimit,
                stat.RxBytes,
                stat.TxBytes,
                created));
        }

        return result;
    }
}

internal readonly record struct SwarmNodeStatsSource(
    Guid PlatformId,
    string DockerNodeId,
    DateTimeOffset ObservedAt);

internal class SendContainersNotificationWorkItem(
    IContainerStreamManager containersStreamManager,
    Guid platformId,
    IReadOnlyList<ContainerStat> stats) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => containersStreamManager.SendContainersStats(platformId, stats);
}
