using System.Threading.Channels;
using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Background job that synchronizes images for all platforms.
/// The job ensures that new images are added, updated images are refreshed, and stale images are removed from the local database.
/// Synchronization occurs in two scenarios:
/// 1. Whenever a platform's status changes (e.g., from offline to online or during recovery).
/// 2. Periodically, every 12 hours, to ensure the local image state remains consistent with the platform state.
/// </summary>
internal class ImageSyncJob(
    IServiceScopeFactory scopeFactory,
    IImageStreamManager imageStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IImageConnector> connectorFactory,
    IPlatformContainerCache platformContainerCache,
    ILogger<ImageSyncJob> logger
) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(12);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{ImageSyncJob} started. Running every {Hours} hours.", nameof(ImageSyncJob), SyncInterval.TotalHours);

        // Event-driven sync starts immediately
        var eventDrivenTask = RunEventDrivenSync(cancellationToken);

        // Periodic sync starts with jitter
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    /// <summary>
    /// Reacts to platform health events and syncs images for platforms as they change state.
    /// </summary>
    private async Task RunEventDrivenSync(CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in _platformHealthReader.ReadAllAsync(cancellationToken))
        {
            try
            {
                await SyncImagesForPlatform(platformEvent, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while syncing images for {Address}", platformEvent.Address);
            }
        }
    }

    /// <summary>
    /// Periodically syncs all platforms every 12 hours.
    /// </summary>
    private async Task RunPeriodicSync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await SyncAllPlatforms(cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during periodic image synchronization.");
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    /// <summary>
    /// Fetches all platforms from the database and syncs them.
    /// </summary>
    private async Task SyncAllPlatforms(CancellationToken cancellationToken)
    {
        if (platformContainerCache.TryGetCacheEntries(out var platforms, out var _))
        {
            foreach (var platform in platforms)
            {
                if (cancellationToken.IsCancellationRequested) break;

                try
                {
                    var platformEvent = new PlatformHealth(
                        Id: platform.Id,
                        Type: platform.ConnectorType,
                        Address: platform.Address,
                        IsOnLine: true
                    );

                    await SyncImagesForPlatform(platformEvent, cancellationToken);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error syncing images for platform {PlatformId}", platform.Id);
                }
            }
        }
    }

    /// <summary>
    /// Core logic to synchronize images for a single platform.
    /// </summary>
    private async Task SyncImagesForPlatform(PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        if (platformEvent.IsOnLine)
        {
            logger.LogInformation("Synchronizing images for platform {PlatformId} at {Address}...", platformEvent.Id, platformEvent.Address);
            var syncedImages = await SyncOnlinePlatformImages(platformEvent, cancellationToken);
            
            await imageStreamManager.SendImagesInfo(platformEvent.Id, syncedImages);
            logger.LogInformation("Synchronized {Count} images for platform {PlatformId}.", syncedImages.Count, platformEvent.Id);
        }
    }

    private async Task<List<Image>> SyncOnlinePlatformImages(PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var result = await connectorFactory
            .GetConnector(platformEvent.Type)
            .ListImagesAsync(platformEvent.Address, cancellationToken: cancellationToken);

        if (!result.IsSuccess(out var freshImages, out var error))
        {
            logger.LogError("Error retrieving image list for platform {PlatformId} at {Address}: {Error}", platformEvent.Id, platformEvent.Address, error);
            return [];
        }

        var images = await uow.Images.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
        var existingImagesInDb = images.ToDictionary(c => c.ImageId, c => c);

        var currentActiveImages = new List<Image>();
        foreach (var freshImage in freshImages)
        {
            if (existingImagesInDb.TryGetValue(freshImage.Id, out var existingDbImage))
            {
                existingDbImage.PartialUpdate(
                    imageId: freshImage.Id,
                    isInUse: freshImage.Containers > 0,
                    tag: freshImage.RepoTags?.Count > 0 ?  freshImage.RepoTags[0] : "",
                    size: freshImage.Size
                );
                currentActiveImages.Add(existingDbImage);
            }
            else
            {
                currentActiveImages.Add(freshImage.Map(platformEvent.Id));
            }
        }

        await uow.Images.BulkUpsertAsync(currentActiveImages, cancellationToken);

        // Remove stale images
        var freshIds = freshImages.Select(f => f.Id).ToHashSet();
        var staleImages = existingImagesInDb.Values.Where(c => !freshIds.Contains(c.ImageId)).ToArray();
        if (staleImages.Length > 0)
        {
            logger.LogInformation("Removing {Count} stale images for platform {PlatformId}", staleImages.Length, platformEvent.Id);
            await uow.Images.DeleteAsync(staleImages.Select(s => s.Id), cancellationToken);
        }

        await uow.CommitAsync();
        return currentActiveImages;
    }
}
