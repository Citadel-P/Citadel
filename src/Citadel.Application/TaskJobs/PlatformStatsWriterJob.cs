using System.Threading.Channels;
using Application.Configs;
using Application.Mappers;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platform statistics received from a channel, 
/// periodically flushing them to the database and notifying connected clients with the latest platformStat updates.
/// </summary>
internal class PlatformStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    IPlatformStreamManager platformStreamManager,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    ILogger<PlatformStatsWriterJob> logger) : BackgroundService
{
    private readonly Dictionary<Guid, List<PlatformStatsResult>> _buffer = [];
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var (platformId, platformStat) in reader.ReadAllAsync(cancellationToken))
            {
                AccumulateBatchStats(platformId, platformStat);
                await platformStreamManager.PushPlatformStats(platformId, platformStat);

                if (ShouldFlush())
                {
                    await FlushToDatabase(cancellationToken);
                }
            }
        }
        catch (OperationCanceledException)
        {
            // expected when service is stopping
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

    private void AccumulateBatchStats(Guid platformId, PlatformStatsResult stat)
    {
        if (!_buffer.TryGetValue(platformId, out var list))
        {
            _buffer[platformId] = list = [];
        }

        list.Add(stat);
    }

    private bool ShouldFlush()
    {
        int totalCount = _buffer.Sum(kvp => kvp.Value.Count);

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
                    containerCount: lastBatch.PlatformStat.ContainerCount,
                    containersRunning: lastBatch.PlatformStat.ContainersRunning,
                    containersPaused: lastBatch.PlatformStat.ContainersPaused,
                    containersStopped: lastBatch.PlatformStat.ContainersStopped);
            }
            else if (existing.PlatformDescriptor is DockerSwarmPlatformDescriptor)
            {
                // Todo
            }
            else if (existing.PlatformDescriptor is KubernetesPlatformDescriptor)
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

        try
        {
            var mappedStats = _buffer.Map();
            await uow.PlatformStats.BulkInsertAsync(mappedStats, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "An error occurred while persisting the platform statistics batch to the database. Total stats in batch: {BatchSize}",
                _buffer.Sum(s => s.Value.Count));
        }
    }
}
