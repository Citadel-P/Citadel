using System.Threading.Channels;
using Application.Configs;
using Application.Services.SignalR;
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
    ChannelReader<ContainersStatBatch> reader,
    IContainersStreamManager containersStreamManager,
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
                AccumulateBatchStats(batch);
                await NotifyClients(batch);

                if (ShouldFlush())
                {
                    await FlushToDatabase(cancellationToken);
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Expected when service stops
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

        list.AddRange(batch.Stats);
    }

    private bool ShouldFlush()
    {
        int totalCount = buffer.Sum(kvp => kvp.Value.Count);
        return totalCount >= options.Value?.BatchSize ||
               (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    }

    private async Task FlushToDatabase(CancellationToken cancellationToken)
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
        finally
        {
            buffer.Clear();
            _lastFlush = DateTime.UtcNow;
        }
    }

    private async ValueTask NotifyClients(ContainersStatBatch batch)
    {
        try
        {
            await containersStreamManager.SendContainersStats(batch.PlatformId, batch.Stats);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to notify clients about container stats for platform {PlatformId}", batch.PlatformId);
        }
    }
}
