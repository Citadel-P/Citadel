using System.Threading.Channels;
using Application.Configs;
using Application.Services.Abstractions;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using static Hosting.Common.Constants;

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
    private readonly Dictionary<Guid, List<ContainerStat>> buffer = [];  // Key: PlatformId
    private DateTime _lastFlush = DateTime.UtcNow;

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
                    batch.Stats.Dispose();
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
            buffer.TryAdd(batch.PlatformId, list = objectPoolManager.Get<List<ContainerStat>>());
        }

        list.AddRange(batch.Stats.Value); // NOTE: only references copied, not the list
    }

    private bool ShouldFlush()
    {
        int totalCount = 0;
        foreach (var kvp in buffer)
        {
            totalCount += kvp.Value.Count;
        }
        return totalCount >= options.Value?.BatchSize || 
            (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    }

    private async Task FlushToDatabase(CancellationToken cancellationToken)
    {
        // Save and then clean up buffer
        await SaveBatchToDb(cancellationToken);
        foreach (var (_, stats) in buffer)
        {
            foreach (var stat in stats)
            {
                objectPoolManager.Return(stat);
            }
        }

        buffer.Clear();
        _lastFlush = DateTime.UtcNow;
    }

    private async Task SaveBatchToDb(CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            var stats = buffer.SelectMany(s => s.Value);
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
        if (connectionTracker.HasUsersInGroup(SignalRGroups.ContainersGroup(batch.PlatformId)))
        {
            try
            {
                await containerHubDispatcher.SendContainersStats(batch.PlatformId, batch.Stats.Value);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to notify clients about containers stats for platform {PlatformId}", batch.PlatformId);
            }
        }
    }
}