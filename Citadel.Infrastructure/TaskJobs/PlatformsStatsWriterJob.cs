using System.Threading.Channels;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Background service that writes batches of platforms stats to the database and notifies clients via SignalR.
/// </summary>
internal class PlatformsStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    ChannelReader<PlatformStatsBatch> reader,
    ISignalRConnectionTracker connectionTracker,
    ILogger<PlatformsStatsWriterJob> logger) : BackgroundService
{
    private const int BatchSize = 200;
    // Updates to db will be flushed every 60 seconds or when batch size is reached.
    private static readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(60); 

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var buffer = new Dictionary<Guid, List<PlatformStat>>(); 
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
                list.Add(batch.Stat);

                // Flush if batch size exceeded or interval exceeded
                int totalCount = buffer.Sum(x => x.Value.Count);
                if (totalCount >= BatchSize || DateTime.UtcNow - lastFlush >= FlushInterval)
                {
                    await SaveBatchToDb(buffer, batch, cancellationToken);
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
            logger.LogError(ex, $"Error in {nameof(PlatformsStatsWriterJob)}");
        }
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<PlatformStat>> statsByPlatform, PlatformStatsBatch batch, CancellationToken cancellationToken)
    {
        using var scope = scopeFactory.CreateAsyncScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        foreach (var (platformId, stats) in statsByPlatform)
        {
            var existing = await db.Platforms.FirstOrDefaultAsync(s => s.Id == platformId, cancellationToken);
            if (existing == null)
            {
                logger.LogWarning("Platform with ID {PlatformId} not found in the database.", platformId);
                continue;
            }

            existing.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networkCount: batch.NetworksCount,
                volumeCount: batch.VolumesCount,
                containersRunning: batch.ContainersRunning,
                containersPaused: batch.ContainersPaused,
                containersStopped: batch.ContainersStopped,
                imageCount: batch.Images,
                memTotal: batch.MemTotal);
        }

        try
        {
            db.PlatformStats.AddRange(statsByPlatform.Values.SelectMany(s => s));
            await db.SaveChangesAsync(cancellationToken);
        }
        catch (DbUpdateException ex)
        {
            logger.LogError(ex, "Failed to save platform stats batch to the database. Batch size: {BatchSize}", statsByPlatform.Sum(s => s.Value.Count));
        }
    }
    private async ValueTask NotifyClients(PlatformStatsBatch batch)
    {
        if (connectionTracker.HasUsersInGroup("Platforms"))
        {
            using var scope = scopeFactory.CreateAsyncScope();
            var hub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();
            try
            {
                await hub.PushPlatformStats(batch);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to push platform stats to clients for platform {PlatformId}", batch.PlatformId);
                return;
            }
        }
    }
}
