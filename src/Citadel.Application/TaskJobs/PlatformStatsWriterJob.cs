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
using System.Buffers;
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
        using var flushTimer = new PeriodicTimer(
            TimeSpan.FromSeconds(Math.Max(1, options.Value.FlashInterval)));

        try
        {
            var readTask = reader.WaitToReadAsync(cancellationToken).AsTask();
            var flushTask = flushTimer.WaitForNextTickAsync(cancellationToken).AsTask();

            while (!cancellationToken.IsCancellationRequested)
            {
                var completed = await Task.WhenAny(readTask, flushTask);
                if (completed == flushTask)
                {
                    if (!await flushTask)
                        break;

                    if (_bufferedCount > 0)
                        await FlushAsync(cancellationToken);

                    flushTask = flushTimer.WaitForNextTickAsync(cancellationToken).AsTask();
                    continue;
                }

                if (!await readTask)
                    break;

                while (reader.TryRead(out var item))
                {
                    Accumulate(item.Id, item.Stats);
                    await EnqueueNotificationAsync(item.Id, item.Stats, cancellationToken);

                    if (ShouldFlush())
                        await FlushAsync(cancellationToken);
                }

                readTask = reader.WaitToReadAsync(cancellationToken).AsTask();
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error in PlatformStatsWriterJob");
        }

        if (!cancellationToken.IsCancellationRequested && _bufferedCount > 0)
            await FlushAsync(cancellationToken);
    }

    private async Task EnqueueNotificationAsync(
        Guid platformId,
        PlatformStatsResult stat,
        CancellationToken cancellationToken)
    {
        if (!platformStreamManager.HasStatsSubscribers)
            return;

        try
        {
            await notificationQueue.EnqueueAsync(
                new SendPlatformNotificationWorkItem(platformStreamManager, stat, platformId),
                cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogWarning(
                ex,
                "Failed to enqueue platform stats notification for platform {PlatformId}.",
                platformId);
        }
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
        var workItem = new PersistPlatformStatsWorkItem(dataToFlush);
        var retryDelay = TimeSpan.FromSeconds(1);

        while (true)
        {
            try
            {
                await dbQueue.EnqueueAndWaitAsync(workItem, ct);
                _buffer = [];
                _bufferedCount = 0;
                _lastFlush = DateTime.UtcNow;
                break;
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Failed to persist platform stats batch. Retrying in {Delay}.",
                    retryDelay);
                await Task.Delay(retryDelay, ct);
                retryDelay = TimeSpan.FromSeconds(Math.Min(30, retryDelay.TotalSeconds * 2));
            }
        }

        if (workItem.AlertContext is { } alertContext)
            await ProcessAlertsAsync(alertContext, ct);
    }

    private async Task ProcessAlertsAsync(
        AlertEvaluationContext context,
        CancellationToken cancellationToken)
    {
        await alertService.ProcessAsync(AlertType.PlatformCpuHigh, context, cancellationToken);
        await alertService.ProcessAsync(AlertType.PlatformRamHigh, context, cancellationToken);
        await alertService.ProcessAsync(AlertType.PlatformDiskHigh, context, cancellationToken);
        await alertService.ProcessAsync(AlertType.PlatformVersionMismatch, context, cancellationToken);
    }
}

internal sealed class PersistPlatformStatsWorkItem(
    Dictionary<Guid, List<PlatformStatsResult>> buffer) : IDbWorkItem
{
    public AlertEvaluationContext? AlertContext { get; private set; }

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

            platformSnapshots.Add(BuildThresholdAlertSnapshot(platformId, existing.Name, stats));

            PlatformDescriptor? descriptor = existing.PlatformDescriptor switch
            {
                DockerSwarmPlatformDescriptor => null, // TODO
                KubernetesPlatformDescriptor => null,  // TODO
                DockerPlatformDescriptor docker => docker.Create(
                    containerCount: last.PlatformStat.ContainerCount,
                    containersRunning: last.PlatformStat.ContainersRunning,
                    containersPaused: last.PlatformStat.ContainersPaused,
                    containersStopped: last.PlatformStat.ContainersStopped
                ) with
                {
                    ImageUsedBytes = last.ImageUsedBytes,
                    VolumeUsedBytes = last.VolumeUsedBytes
                },
                _ => null
            };

            existing.PartialUpdate(
                networkCount: last.NetworkCount,
                volumeCount: last.VolumeCount,
                imageCount: last.ImageCount,
                memTotal: last.MemTotal,
                descriptor: descriptor);

            await uow.Platforms.UpdateAsync(existing, cancellationToken);
        }

        var mapped = filteredBuffer.Map();
        if (mapped.Count != 0)
        {
            await uow.PlatformStats.BulkInsertAsync(mapped, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        if (platformSnapshots.Count != 0)
        {
            AlertContext = new AlertEvaluationContext(
                UtcNow: now,
                Platforms: platformSnapshots,
                Deployments: [],
                Stacks: []);
        }
    }

    internal static PlatformAlertSnapshot BuildThresholdAlertSnapshot(
        Guid platformId,
        string platformName,
        IReadOnlyList<PlatformStatsResult> stats)
    {
        var last = stats[^1];
        var diskUsage = last.PlatformStat.DiskUsage;
        var diskUsedBytes = last.PlatformStat.DiskUsedBytes;
        var diskTotalBytes = last.PlatformStat.DiskTotalBytes;
        var hasValidDiskSample = diskUsage is { } usage
                                 && double.IsFinite(usage)
                                 && usage is >= 0 and <= 100
                                 && diskUsedBytes is >= 0
                                 && diskTotalBytes is > 0
                                 && diskUsedBytes <= diskTotalBytes;

        return new PlatformAlertSnapshot(
            platformId,
            platformName,
            CpuUsage: Median(stats, static x => x.PlatformStat.CpuUsage),
            RamUsage: Median(stats, static x => x.PlatformStat.MemoryUsage),
            AgentVersion: last.AgentVersion,
            DiskUsage: hasValidDiskSample ? diskUsage : null,
            DiskUsedBytes: hasValidDiskSample ? diskUsedBytes : null,
            DiskTotalBytes: hasValidDiskSample ? diskTotalBytes : null);
    }

    private static double Median(
        IReadOnlyList<PlatformStatsResult> values,
        Func<PlatformStatsResult, double> selector)
    {
        var rented = ArrayPool<double>.Shared.Rent(values.Count);
        try
        {
            var count = 0;
            foreach (var item in values)
            {
                var value = selector(item);
                if (double.IsFinite(value))
                    rented[count++] = value;
            }

            if (count == 0)
                return 0;

            var ordered = rented.AsSpan(0, count);
            ordered.Sort();
            return count % 2 == 1
                ? ordered[count / 2]
                : (ordered[(count / 2) - 1] + ordered[count / 2]) / 2.0;
        }
        finally
        {
            ArrayPool<double>.Shared.Return(rented);
        }
    }
}

internal class SendPlatformNotificationWorkItem(IPlatformStreamManager platformStreamManager, PlatformStatsResult stat, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => platformStreamManager.PushPlatformStats(platformId, stat);
}
