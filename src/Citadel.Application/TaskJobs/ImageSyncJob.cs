using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Background job that synchronizes images for all platforms.
/// The job ensures that new images are added, updated images are refreshed, and stale images are removed from the local database.
/// Synchronization occurs in two scenarios:
/// 1. Whenever a platform's status changes (e.g., from offline to online or during recovery).
/// 2. Periodically, every 6 hours, to ensure the local image state remains consistent with the platform state.
/// </summary>
internal class ImageSyncJob(
    IDbWorkQueue dbQueue,
    ISyncBarrier syncBarrier,
    INotificationQueue notifQueue,
    IImageStreamManager imageStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IImageConnector> connectorFactory,
    IPlatformContainerCache platformContainerCache,
    ILogger<ImageSyncJob> logger
) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{ImageSyncJob} started. Runs every {Hours} hours.",
            nameof(ImageSyncJob), SyncInterval.TotalHours);

        var eventDriven = RunEventDrivenSync(cancellationToken);
        var periodic = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDriven, periodic);
    }

    private async Task RunEventDrivenSync(CancellationToken ct)
    {
        await foreach (var platform in _platformHealthReader.ReadAllAsync(ct))
        {
            try
            {
                // Schedule sync via DB queue
                var result = await connectorFactory.GetConnector(platform.Type).ListImagesAsync(platform.Address, cancellationToken: ct);
                if (!result.IsSuccess(out var freshImages, out var error))
                {
                    logger.LogError("failed to list images for {Platform}", platform.Address);
                    continue;
                }
                await dbQueue.EnqueueAsync(new ImageSyncWorkItem(freshImages, imageStreamManager, notifQueue, syncBarrier, platform, logger), ct);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "image sync schedule failed");
            }
        }
    }

    private async Task RunPeriodicSync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                if (platformContainerCache.TryGetCacheEntries(out var platforms, out _))
                {
                    foreach (var p in platforms)
                    {
                        // Fetch images from Docker
                        var result = await connectorFactory.GetConnector(p.ConnectorType).ListImagesAsync(p.Address, cancellationToken: ct);
                        if (!result.IsSuccess(out var freshImages, out var error))
                        {
                            logger.LogError("failed to list images for {Platform}", p.Address);
                            continue;
                        }
                        if (ct.IsCancellationRequested) break;
                        await dbQueue.EnqueueAsync(new ImageSyncWorkItem(freshImages, imageStreamManager, notifQueue, syncBarrier, new PlatformHealth(p.Id, p.Address, p.ConnectorType, true), logger), ct);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "periodic image sync failed");
            }

            await Task.Delay(SyncInterval, ct);
        }
    }
}

internal sealed class ImageSyncWorkItem(
    IEnumerable<ImageResult> freshImages,
    IImageStreamManager imageStreamManager,
    INotificationQueue notificationQueue,
    ISyncBarrier syncBarrier,
    PlatformHealth platform,
    ILogger logger
) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var images = await uow.Images.GetByPlatformIdAsync(platform.Id, ct);
        var existing = images.ToDictionary(x => x.DockerImageId);

        var upserts = new List<Image>();

        foreach (var fresh in freshImages)
        {
            if (existing.TryGetValue(fresh.Id, out var dbImage))
            {
                dbImage.PartialUpdate(
                    dockerImageId: fresh.Id,
                    containers: fresh.Containers,
                    tag: fresh.RepoTags?.FirstOrDefault() ?? "",
                    size: fresh.Size
                );
                upserts.Add(dbImage);
            }
            else
            {
                upserts.Add(fresh.Map(platform.Id));
            }
        }

        await uow.Images.BulkUpsertAsync(upserts, ct);

        // Delete stale
        var freshIds = freshImages.Select(f => f.Id).ToHashSet();
        var stale = existing.Values.Where(x => !freshIds.Contains(x.DockerImageId)).ToArray();

        if (stale.Length > 0)
            await uow.Images.DeleteAsync(stale.Select(s => s.Id), ct);

        await uow.CommitAsync(ct);

        // Notify clients
        await notificationQueue.EnqueueAsync(new SendImagesNotificationWorkItem(imageStreamManager, upserts, platform.Id), ct);

        // Mark first successful sync
        syncBarrier.MarkSynced<ImageSyncJob>();
    }
}

internal sealed class SendImagesNotificationWorkItem(IImageStreamManager imageStreamManager, IEnumerable<Image> images, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken ct)
        => imageStreamManager.SendImagesInfo(platformId, images);
}
