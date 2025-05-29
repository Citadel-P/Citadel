using System.Threading.Channels;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Background service that writes batches of container stats to the database and notifies clients via SignalR.
/// </summary>
internal class ContainersStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    ChannelReader<ContainersStatBatch> reader,
    ISignalRConnectionTracker connectionTracker,
    ILogger<ContainersStatsWriterJob> logger) : BackgroundService
{
    private const int BatchSize = 200;
    // Updates to db will be flushed every 60 seconds or when batch size is reached.
    private static readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(60); 

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var buffer = new Dictionary<Guid, List<ContainerStat>>(); // Key: PlatformId
        var lastFlush = DateTime.UtcNow;

        try
        {
            await foreach (var batch in reader.ReadAllAsync(cancellationToken))
            {
                // Add to buffer
                if (!buffer.TryGetValue(batch.PlatformId, out var list))
                {
                    list = [];
                    buffer[batch.PlatformId] = list;
                }
                list.AddRange(batch.Stats);

                // Flush if batch size exceeded or interval exceeded
                int totalCount = buffer.Sum(x => x.Value.Count);
                if (totalCount >= BatchSize || DateTime.UtcNow - lastFlush >= FlushInterval)
                {
                    await SaveBatchToDb(buffer, cancellationToken);
                    buffer.Clear();
                    lastFlush = DateTime.UtcNow;
                }

                // Push to clients
                await NotifyClients(batch);
            }
        }
        catch (OperationCanceledException)
        {
            // Nope, this is expected when the service is stopping
        }
        catch (Exception ex)
        {
            logger.LogError(ex, $"Error in {nameof(ContainersStatsWriterJob)}");
        }

        // Final flush
        if (buffer.Count > 0)
        {
            await SaveBatchToDb(buffer, CancellationToken.None);
        }
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<ContainerStat>> statsByPlatform, CancellationToken cancellationToken)
    {
        using var scope = scopeFactory.CreateAsyncScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var stats = statsByPlatform.SelectMany(s => s.Value).ToList();
        try
        {
            db.ContainerStats.AddRange(stats);
            await db.SaveChangesAsync(cancellationToken);
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
            using var scope = scopeFactory.CreateAsyncScope();
            var hub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();
            try
            {
                await hub.SendContainersStats(batch.PlatformId, batch.Stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to notify clients about containers stats for platform {PlatformId}", batch.PlatformId);
            }
        }
    }
}
