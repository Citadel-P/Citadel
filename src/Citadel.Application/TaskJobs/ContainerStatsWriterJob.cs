using System.Threading.Channels;
using Application.Configs;
using Application.Services.Abstractions;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal class ContainerStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    IObjectPoolManager objectPoolManager,
    ChannelReader<ContainersStatBatch> reader,
    ISignalRConnectionTracker connectionTracker,
    IContainerHubDispatcher containerHubDispatcher,
    ILogger<ContainerStatsWriterJob> logger) : BackgroundService
{
    // Updates to db will be flushed every x seconds or when batch size is reached.
    private readonly TimeSpan flushInterval = TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    private readonly int batchSize = options.Value?.BatchSize ?? 100;
    private readonly Dictionary<Guid, List<ContainerStat>> buffer = [];  // Key: PlatformId
    private DateTime lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var batch in reader.ReadAllAsync(cancellationToken))
            {
                try
                {
                    AccumulateBatchStats(batch);
                    await NotifyClients(batch);

                    if (ShouldFlush())
                    {
                        await FlushToDatabase(cancellationToken);
                    }
                }
                finally
                {
                    // Return stats to pool
                    batch.Stats.Clear();
                    objectPoolManager.Return(batch.Stats);
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Nope, this is expected when the service is stopping
        }
        catch (Exception ex)
        {
            logger.LogError(ex, $"Error in {nameof(ContainerStatsWriterJob)}");
        }
        
        // Final flush if needed
        if (buffer.Count > 0)
        {
            await FlushToDatabase(cancellationToken);
        }
    }

    private void AccumulateBatchStats(ContainersStatBatch batch)
    {
        if (!buffer.TryGetValue(batch.PlatformId, out var list))
        {
            buffer.TryAdd(batch.PlatformId, list = []);
        }

        list.AddRange(batch.Stats); // NOTE: only references copied, not the list
    }

    private bool ShouldFlush()
    {
        int totalCount = 0;
        foreach (var kvp in buffer)
        {
            totalCount += kvp.Value.Count;
        }
        return totalCount >= batchSize || DateTime.UtcNow - lastFlush >= flushInterval;
    }

    
    private async Task FlushToDatabase(CancellationToken cancellationToken)
    {
        // Save and then clean up buffer
        await SaveBatchToDb(buffer, cancellationToken);
        foreach (var (_, stats) in buffer)
        {
            foreach (var stat in stats)
            {
                objectPoolManager.Return(stat);
            }

            stats.Clear();
            objectPoolManager.Return(stats);
        }

        buffer.Clear();
        lastFlush = DateTime.UtcNow;
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<ContainerStat>> statsByPlatform, CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            var stats = statsByPlatform.SelectMany(s => s.Value);
            await uow.ContainerStats.BulkInsertAsync(stats, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        { 
            logger.LogError(ex, "Failed to save container stats to the database.");
        }
    }

    private async ValueTask NotifyClients(ContainersStatBatch batch)
    {
        if (connectionTracker.HasUsersInGroup($"ContainersInfo/{batch.PlatformId}"))
        {
            try
            {
                await containerHubDispatcher.SendContainersStats(batch.PlatformId, batch.Stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to notify clients about containers stats for platform {PlatformId}", batch.PlatformId);
            }
        }
    }
}