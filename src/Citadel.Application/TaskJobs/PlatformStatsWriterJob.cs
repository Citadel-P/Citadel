using Application.Configs;
using Application.Mappers;
using Application.Services.Alerts;
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
internal sealed class PlatformStatsWriterJob(
    IDbWorkQueue dbQueue,
    IAlertService alertService,
    INotificationQueue notificationQueue,
    IPlatformStreamManager platformStreamManager,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    IOptions<JobConfiguration> options,
    ILogger<PlatformStatsWriterJob> logger
) : BackgroundService
{
    private int _bufferedCount;
    private DateTime _lastFlush = DateTime.UtcNow;
    private Dictionary<Guid, List<PlatformStatsResult>> _buffer = [];

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var (platformId, stat) in reader.ReadAllAsync(cancellationToken))
            {
                Accumulate(platformId, stat);

                // Push notification
                await notificationQueue.EnqueueAsync(
                    new SendPlatformNotificationWorkItem(platformStreamManager, stat, platformId),
                    cancellationToken);

                if (ShouldFlush())
                    await FlushAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error in PlatformStatsWriterJob");
        }

        if (_bufferedCount > 0)
            await FlushAsync(CancellationToken.None);
    }

    private void Accumulate(Guid id, PlatformStatsResult stat)
    {
        if (!_buffer.TryGetValue(id, out var list))
            _buffer[id] = list = [];

        list.Add(stat);
        _bufferedCount++;
    }

    private bool ShouldFlush() =>
        _bufferedCount >= options.Value.BatchSize ||
        (DateTime.UtcNow - _lastFlush) >= TimeSpan.FromSeconds(options.Value.FlashInterval);

    private async Task FlushAsync(CancellationToken ct)
    {
        if (_bufferedCount == 0) return;

        var dataToFlush = _buffer;
        _buffer = new Dictionary<Guid, List<PlatformStatsResult>>();

        _bufferedCount = 0;
        _lastFlush = DateTime.UtcNow;

        try
        {
            await dbQueue.EnqueueAsync(new PersistPlatformStatsWorkItem(dataToFlush, alertService, logger), ct);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to enqueue platform stats persist work item.");
        }
    }
}

internal sealed class PersistPlatformStatsWorkItem(
    Dictionary<Guid, List<PlatformStatsResult>> buffer,
    IAlertService alertService,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var now = DateTime.UtcNow;
        var platformSnapshots = new List<PlatformAlertSnapshot>();
        var filteredBuffer = new Dictionary<Guid, List<PlatformStatsResult>>();

        foreach (var (platformId, stats) in buffer)
        {
            var existing = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            if (existing == null) continue;

            var last = stats.LastOrDefault();
            if (last == null) continue;

            filteredBuffer[platformId] = stats;

            foreach (var stat in stats)
            {
                platformSnapshots.Add(new PlatformAlertSnapshot(
                   platformId,
                   existing.Name,
                   CpuUsage: stat.PlatformStat.CpuUsage,
                   RamUsage: stat.PlatformStat.MemoryUsage,
                   AgentVersion: stat.AgentVersion));
            }

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

        // Bulk Insert Historical Stats
        try
        {
            var mapped = filteredBuffer.Map();
            if (mapped.Count != 0)
            {
                await uow.PlatformStats.BulkInsertAsync(mapped, cancellationToken);
                await uow.CommitAsync(cancellationToken);
            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error bulk inserting PlatformStats");
        }

        // Alert processing
        if (platformSnapshots.Count != 0)
        {
            var context = new AlertEvaluationContext(
                UtcNow: now,
                Platforms: platformSnapshots,
                Deployments: [],
                Stacks: []);

            await alertService.ProcessAsync(
                AlertType.PlatformCpuHigh,
                context,
                cancellationToken);

            await alertService.ProcessAsync(
                AlertType.PlatformRamHigh,
                context,
                cancellationToken);

            await alertService.ProcessAsync(
                AlertType.PlatformVersionMismatch,
                context,
                cancellationToken);
        }
    }
}

internal class SendPlatformNotificationWorkItem(IPlatformStreamManager platformStreamManager, PlatformStatsResult stat, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => platformStreamManager.PushPlatformStats(platformId, stat);
}