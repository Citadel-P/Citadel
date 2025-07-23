using System.Threading.Channels;
using Application.Configs;
using Application.Mappers;
using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platforms statistics received from a channel, periodically flushing them to the database and notifying connected clients 
/// with the latest platformStat updates.
/// </summary>
internal class PlatformStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    IObjectPoolManager objectPoolManager,
    ISignalRConnectionTracker connectionTracker,
    IPlatformHubDispatcher platformHubDispatcher,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    ILogger<PlatformStatsWriterJob> logger) : BackgroundService
{
    // Updates to db will be flushed every x seconds or when platform batch size is reached.
    private readonly TimeSpan flushInterval = TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    private readonly int batchSize = options.Value?.BatchSize ?? 100;
    private readonly Dictionary<Guid, List<PlatformStatsResult>> buffer = [];
    private DateTime lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var (platformId, platformStat) in reader.ReadAllAsync(cancellationToken))
            {
                AccumulateBatchStats(platformId, platformStat);
                await NotifyClients(platformId, platformStat);

                if (ShouldFlush())
                {
                    await FlushToDatabase(cancellationToken);
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Nope, this is expected when the service is stopping
        }
        catch (Exception ex)
        {
            logger.LogError(ex, $"Error in {nameof(PlatformStatsWriterJob)}");
        }

        // Final flush
        if (buffer.Count > 0)
        {
            await SaveBatchToDb(buffer, CancellationToken.None);
        }
    }

    private void AccumulateBatchStats(Guid platformId, PlatformStatsResult stat)
    {
        if (!buffer.TryGetValue(platformId, out var list))
        {
            buffer.TryAdd(platformId, list = objectPoolManager.Get<List<PlatformStatsResult>>());
        }

        list.Add(stat); // NOTE: only references copied, not the list
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

    private async Task SaveBatchToDb(Dictionary<Guid, List<PlatformStatsResult>> statsByPlatform, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        foreach (var (platformId, stats) in statsByPlatform)
        {
            var existing = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            if (existing == null)
            {
                logger.LogWarning("Platform with ID {PlatformId} not found in the database.", platformId);
                continue;
            }

            var lastBatch = stats.LastOrDefault();
            if (lastBatch == null)
                continue;

            PlatformDescriptor? descriptor = null;
            if (existing.PlatformDescriptor is DockerPlatformDescriptor dockerPlatform)
            {
                descriptor = dockerPlatform.Create(
                    containersRunning: lastBatch.ContainersRunning,
                    containersPaused: lastBatch.ContainersPaused,
                    containersStopped: lastBatch.ContainersStopped);
            }
            else if (existing.PlatformDescriptor is DockerSwarmPlatformDescriptor swarmDescriptor)
            {
                // Todo
            }
            else if (existing.PlatformDescriptor is KubernetesPlatformDescriptor kubernetesDescriptor)
            {
                // Todo
            }

            existing.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networkCount: lastBatch.NetworkCount,
                volumeCount: lastBatch.VolumeCount,
                imageCount: lastBatch.ImageCount,
                memTotal: lastBatch.MemTotal,
                descriptor: descriptor);

            await uow.Platforms.UpdatePlatformAsync(existing, cancellationToken);
        }

        await uow.CommitAsync();
        var pooledStats = MapToStats(statsByPlatform);
        
        try
        {
            await uow.PlatformStats.BulkInsertAsync(pooledStats, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while persisting the platform statistics batch to the database. Total stats in batch: {BatchSize}", statsByPlatform.Sum(s => s.Value.Count));
        }
        finally
        {
            foreach (var stat in pooledStats)
            {
                objectPoolManager.Return(stat);
            }

            objectPoolManager.Return(pooledStats);
        }
    }

    private List<PlatformStat> MapToStats(Dictionary<Guid, List<PlatformStatsResult>> statsByPlatform)
    {
        var stats = objectPoolManager.Get<List<PlatformStat>>();

        foreach (var (platformId, platformStats) in statsByPlatform)
        {
            foreach (var stat in platformStats)
            {
                var destination = objectPoolManager.Get<PlatformStat>();
                stat.Map(destination, platformId);
                stats.Add(destination);
            }
        }
        return stats;
    }

    private async ValueTask NotifyClients(Guid platformId, PlatformStatsResult stats)
    {
        if (connectionTracker.HasUsersInGroup("Platforms"))
        {
            try
            {
                await platformHubDispatcher.PushPlatformStats(platformId, stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "An error occurred while sending platform statistics to clients for platform ID: {PlatformId}", platformId);
                return;
            }
        }
    }
}
