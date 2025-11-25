using Application.Configs;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal class ContainerStatsWriterJob(
    IDbWorkQueue dbQueue,
    INotificationQueue notificationQueue,
    IOptions<JobConfiguration> options,
    ChannelReader<ContainersStatBatch> reader,
    IContainerStreamManager containersStreamManager,
    ILogger<ContainerStatsWriterJob> logger
) : BackgroundService
{
    private readonly Dictionary<Guid, List<ContainerStat>> _buffer = [];
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var batch in reader.ReadAllAsync(cancellationToken))
            {
                Accumulate(batch);

                // Push to notification queue
                var notificationWorkItem = new SendContainersNotificationWorkItem(containersStreamManager, batch);
                await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

                if (ShouldFlush())
                    await FlushAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, $"Error in {nameof(ContainerStatsWriterJob)}");
        }

        // final flush
        if (_buffer.Count > 0)
            await FlushAsync(CancellationToken.None);
    }

    private void Accumulate(ContainersStatBatch batch)
    {
        if (!_buffer.TryGetValue(batch.PlatformId, out var list))
            _buffer[batch.PlatformId] = list = [];

        list.AddRange(batch.Stats);
    }

    private bool ShouldFlush()
    {
        int count = _buffer.Sum(kvp => kvp.Value.Count);

        return count >= options.Value?.BatchSize ||
               (DateTime.UtcNow - _lastFlush) >=
                 TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    }

    private async Task FlushAsync(CancellationToken ct)
    {
        try
        {
            // snapshot the buffer to avoid mutation while queued
            var snapshot = _buffer.ToDictionary(
                kvp => kvp.Key,
                kvp => kvp.Value.ToList()
            );

            await dbQueue.EnqueueAsync(new ContainerStatsBatchWorkItem(snapshot, logger), ct);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to enqueue container stats batch.");
        }
        finally
        {
            _buffer.Clear();
            _lastFlush = DateTime.UtcNow;
        }
    }
}

internal sealed class ContainerStatsBatchWorkItem(IReadOnlyDictionary<Guid, List<ContainerStat>> batch, ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken token)
    {
        try
        {
            var flatList = batch.SelectMany(x => x.Value);

            await uow.ContainerStats.BulkInsertAsync(flatList, token);
            await uow.CommitAsync(token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to persist container stats batch.");
        }
    }
}

internal class SendContainersNotificationWorkItem(IContainerStreamManager containersStreamManager, ContainersStatBatch batch): INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => containersStreamManager.SendContainersStats(batch.PlatformId, batch.Stats);
}