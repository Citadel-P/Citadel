using Application.Configs;
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
    IContainerStreamManager containersStreamManager,
    ILogger<ContainerStatsWriterJob> logger) : BackgroundService
{
    private readonly JobConfiguration _config = options.Value;
    private Dictionary<Guid, List<ContainerStat>> _buffer = [];
    private int _bufferedCount;
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var batch in reader.ReadAllAsync(cancellationToken))
            {
                Accumulate(batch);

                // We create a new list here so SignalR doesn't point to a pooled list that gets cleared
                var notificationStats = batch.Stats.ToList();
                await notificationQueue.EnqueueAsync(new SendContainersNotificationWorkItem(
                    containersStreamManager, batch.PlatformId, notificationStats), cancellationToken);

                // Release the pooled list back to the Streamer as fast as possible
                batch.Release();

                if (ShouldFlush())
                    await FlushAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) { }

        // final flush
        if (_bufferedCount > 0)
            await FlushAsync(CancellationToken.None);
    }

    private void Accumulate(ContainersStatBatch batch)
    {
        if (!_buffer.TryGetValue(batch.PlatformId, out var list))
        {
            list = [];
            _buffer[batch.PlatformId] = list;
        }

        list.AddRange(batch.Stats);
        _bufferedCount += batch.Stats.Count;
    }

    private bool ShouldFlush() =>
        _bufferedCount >= _config.BatchSize ||
        (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(_config.FlashInterval);

    private async Task FlushAsync(CancellationToken ct)
    {
        if (_bufferedCount == 0) return;

        // SWAP Strategy: Capture current buffer and replace with a fresh one
        // This ensures the DB worker has its own private copy that won't be modified
        var dataToFlush = _buffer;
        _buffer = new Dictionary<Guid, List<ContainerStat>>();

        _bufferedCount = 0;
        _lastFlush = DateTime.UtcNow;

        try
        {
            await dbQueue.EnqueueAsync(new ContainerStatsBatchWorkItem(dataToFlush, platformContainerCache, logger), ct);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to enqueue DB batch");
        }
    }
}

internal sealed class ContainerStatsBatchWorkItem(
    IReadOnlyDictionary<Guid, List<ContainerStat>> batch,
    IPlatformContainerCache platformContainerCache,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
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
                    {
                        filteredList.Add(stat);
                    }
                    else
                    {
                        droppedCount++;
                    }
                }
            }

            if (filteredList.Count == 0) return;

            if (droppedCount > 0)
            {
                logger.LogWarning(
                    "Dropped {Count} container stats because their containers were no longer present in the platform cache when the batch was flushed.",
                    droppedCount);
            }

            await uow.ContainerStats.BulkInsertAsync(filteredList, token);
            await uow.CommitAsync(token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Database bulk insert failed.");
        }
    }
}

internal class SendContainersNotificationWorkItem(
    IContainerStreamManager containersStreamManager,
    Guid platformId,
    List<ContainerStat> stats) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => containersStreamManager.SendContainersStats(platformId, stats);
}