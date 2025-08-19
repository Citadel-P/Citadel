using System.Threading.Channels;
using Application.Configs;
using Application.Mappers;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common.ObjectPoolManager;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platform statistics received from a channel, periodically flushing them to the database and notifying connected clients 
/// with the latest platformStat updates.
/// </summary>
internal class PlatformStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    IObjectPoolManager objectPoolManager,
    IPlatformsStreamManager platformStreamManager,
    ChannelReader<(Guid Id, PooledHandle<PlatformStatsResult> Stats)> reader,
    ILogger<PlatformStatsWriterJob> logger) : BackgroundService
{
    private readonly Dictionary<Guid, List<PooledHandle<PlatformStatsResult>>> _buffer = [];
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var (platformId, platformStat) in reader.ReadAllAsync(cancellationToken))
            {
                // We delay the disposal of the pooled handle until we have processed it
                AccumulateBatchStats(platformId, platformStat);
                await platformStreamManager.PushPlatformStats(platformId, platformStat.Value);

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
        if (_buffer.Count > 0)
        {
            await SaveBatchToDb(CancellationToken.None);
        }
    }

    private void AccumulateBatchStats(Guid platformId, PooledHandle<PlatformStatsResult> stat)
    {
        if (!_buffer.TryGetValue(platformId, out var list))
        {
            _buffer.TryAdd(platformId, list = objectPoolManager.Get<List<PooledHandle<PlatformStatsResult>>>());
        }

        list.Add(stat); // NOTE: only references copied, not the list
    }

    private bool ShouldFlush()
    {
        int totalCount = 0;
        foreach (var kvp in _buffer)
        {
            totalCount += kvp.Value.Count;
        }

        return totalCount >= options.Value?.BatchSize || 
            (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    }

    private async Task FlushToDatabase(CancellationToken cancellationToken)
    {
        try
        {
            await SaveBatchToDb(cancellationToken);
        }
        finally
        {
            // Clean up
            foreach (var (_, stats) in _buffer)
            {
                foreach (var stat in stats)
                {
                    stat.Dispose();
                }

                stats.Clear();
                objectPoolManager.Return(stats);
            }

            _buffer.Clear();
            _lastFlush = DateTime.UtcNow;
        }
    }

    private async Task SaveBatchToDb(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        foreach (var (platformId, stats) in _buffer)
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
                    containerCount: lastBatch.Value.PlatformStat.ContainerCount,
                    containersRunning: lastBatch.Value.PlatformStat.ContainersRunning,
                    containersPaused: lastBatch.Value.PlatformStat.ContainersPaused,
                    containersStopped: lastBatch.Value.PlatformStat.ContainersStopped);
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
                networkCount: lastBatch.Value.NetworkCount,
                volumeCount: lastBatch.Value.VolumeCount,
                imageCount: lastBatch.Value.ImageCount,
                memTotal: lastBatch.Value.MemTotal,
                descriptor: descriptor);

            await uow.Platforms.UpdatePlatformAsync(existing, cancellationToken);
        }

        using var pooledStats = MapToStats();
        try
        {
            await uow.PlatformStats.BulkInsertAsync(pooledStats.Value, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while persisting the platform statistics batch to the database. Total stats in batch: {BatchSize}", _buffer.Sum(s => s.Value.Count));
        }
        finally
        {
            foreach (var stat in pooledStats.Value)
            {
                objectPoolManager.Return(stat);
            }
            objectPoolManager.Return(pooledStats.Value);
        }
    }

    private PooledHandle<List<PlatformStat>> MapToStats()
    {
        var stats = objectPoolManager.GetPooled<List<PlatformStat>>();

        foreach (var (platformId, platformStats) in _buffer)
        {
            foreach (var stat in platformStats)
            {
                var destination = objectPoolManager.Get<PlatformStat>();
                stat.Value.Map(destination, platformId);
                stats.Value.Add(destination);
            }
        }
        return stats;
    }
}
