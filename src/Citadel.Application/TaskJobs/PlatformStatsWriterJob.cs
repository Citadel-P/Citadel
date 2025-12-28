using Application.Configs;
using Application.Mappers;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platform statistics received from a channel, 
/// periodically flushing them to the database and notifying connected clients with the latest platformStat updates.
/// </summary>
internal class PlatformStatsWriterJob(
    IDbWorkQueue dbQueue,
    INotificationQueue notificationQueue,
    IPlatformStreamManager platformStreamManager,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    IOptions<JobConfiguration> options,
    ILogger<PlatformStatsWriterJob> logger
) : BackgroundService
{
    private readonly Dictionary<Guid, List<PlatformStatsResult>> _buffer = [];
    private DateTime _lastFlush = DateTime.UtcNow;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var (platformId, stat) in reader.ReadAllAsync(cancellationToken))
            {
                Accumulate(platformId, stat);

                // Push to notification queue
                var notificationWorkItem = new SendPlatformNotificationWorkItem(platformStreamManager, stat, platformId);
                await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

                if (ShouldFlush())
                {
                    await FlushAsync(cancellationToken);
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error in PlatformStatsWriterJob");
        }

        if (_buffer.Count > 0)
            await FlushAsync(CancellationToken.None);
    }

    private void Accumulate(Guid id, PlatformStatsResult stat)
    {
        if (!_buffer.TryGetValue(id, out var list))
            _buffer[id] = list = [];

        list.Add(stat);
    }

    private bool ShouldFlush()
    {
        var count = _buffer.Sum(x => x.Value.Count);
        return count >= options.Value?.BatchSize ||
               (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60);
    }

    private async Task FlushAsync(CancellationToken cancellationToken)
    {
        try
        {
            var copy = new Dictionary<Guid, List<PlatformStatsResult>>(_buffer);
            await dbQueue.EnqueueAsync(new PersistPlatformStatsWorkItem(copy, logger), cancellationToken);
        }
        finally
        {
            _buffer.Clear();
            _lastFlush = DateTime.UtcNow;
        }
    }
}


internal sealed class PersistPlatformStatsWorkItem(Dictionary<Guid, List<PlatformStatsResult>> buffer, ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        foreach (var (platformId, stats) in buffer)
        {
            var existing = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            if (existing == null)
            {
                logger.LogWarning("Platform with ID {PlatformId} not found in DB.", platformId);
                continue;
            }

            var last = stats.LastOrDefault();
            if (last == null) continue;

            PlatformDescriptor? descriptor = existing.PlatformDescriptor switch
            {
                DockerSwarmPlatformDescriptor => null, // TODO
                KubernetesPlatformDescriptor => null,  // TODO
                DockerPlatformDescriptor docker => docker.Create(
                    containerCount: last.PlatformStat.ContainerCount,
                    containersRunning: last.PlatformStat.ContainersRunning,
                    containersPaused: last.PlatformStat.ContainersPaused,
                    containersStopped: last.PlatformStat.ContainersStopped
                ),
                
                _ => null
            };

            existing.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networkCount: last.NetworkCount,
                volumeCount: last.VolumeCount,
                imageCount: last.ImageCount,
                memTotal: last.MemTotal,
                descriptor: descriptor);

            await uow.Platforms.UpdateAsync(existing, cancellationToken);
        }

        try
        {
            var mapped = buffer.Map();
            await uow.PlatformStats.BulkInsertAsync(mapped, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while persisting PlatformStats batch (size: {Size})",
                buffer.Sum(s => s.Value.Count));
        }
    }
}

internal class SendPlatformNotificationWorkItem(IPlatformStreamManager platformStreamManager, PlatformStatsResult stat, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => platformStreamManager.PushPlatformStats(platformId, stat);
}