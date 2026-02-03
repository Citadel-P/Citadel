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
    INotificationQueue notificationQueue,
    IOptions<JobConfiguration> options,
    ChannelReader<ContainersStatBatch> reader,
    IContainerStreamManager containersStreamManager,
    ILogger<ContainerStatsWriterJob> logger) : BackgroundService
{
    private readonly JobConfiguration _config = options.Value;
    private Dictionary<Guid, List<ContainerStat>> _buffer = new();
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
                _ = notificationQueue.EnqueueAsync(new SendContainersNotificationWorkItem(
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
            await dbQueue.EnqueueAsync(new ContainerStatsBatchWorkItem(dataToFlush, logger), ct);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to enqueue DB batch");
        }
    }
}

internal sealed class ContainerStatsBatchWorkItem(
    IReadOnlyDictionary<Guid, List<ContainerStat>> batch,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
        {
            // Flatten the dictionary into a single list for bulk insert
            var flatList = batch.Values.SelectMany(x => x).ToList();
            if (flatList.Count == 0) return;

            await uow.ContainerStats.BulkInsertAsync(flatList, token);
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