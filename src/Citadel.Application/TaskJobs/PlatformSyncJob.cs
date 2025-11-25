using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Synchronizes platform state.
/// </summary>
internal class PlatformSyncJob(
    IDbWorkQueue dbWorkQueue,
    INotificationQueue notifQueue,
    IServiceScopeFactory scopeFactory,
    IPlatformStreamManager platformStreamManager,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformSyncJob> logger
) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{PlatformSyncJob} started. Running every {Hours} hours.", nameof(PlatformSyncJob), SyncInterval.TotalHours);

        // Event-driven sync starts immediately
        var eventDrivenTask = RunEventDrivenSync(cancellationToken);

        // Periodic sync starts with jitter
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    /// <summary>
    /// Reacts to platform health events and syncs platforms as they change state.
    /// </summary>
    private async Task RunEventDrivenSync(CancellationToken cancellationToken)
    {
        await foreach (var evt in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            try
            {
                await SyncPlatform(evt, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while synchronizing platform {Address}", evt.Address);
            }
        }
    }

    /// <summary>
    /// Periodically syncs all platforms every x hours.
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
                logger.LogError(ex, "Error during periodic platform synchronization.");
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    /// <summary>
    /// Fetches all platforms from the cache and syncs them.
    /// </summary>
    private async Task SyncAllPlatforms(CancellationToken cancellationToken)
    {
        if (platformContainerCache.TryGetCacheEntries(out var platforms, out _))
        {
            foreach (var platform in platforms)
            {
                if (cancellationToken.IsCancellationRequested)
                    break;

                try
                {
                    var platformEvent = new PlatformHealth(
                        Id: platform.Id,
                        Type: platform.ConnectorType,
                        Address: platform.Address,
                        IsOnLine: true
                    );

                    await SyncPlatform(platformEvent, cancellationToken);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error syncing platform {PlatformId}", platform.Id);
                }
            }
        }
    }

    /// <summary>
    /// Core logic to synchronize a single platform.
    /// </summary>
    private async Task SyncPlatform(PlatformHealth evt, CancellationToken cancellationToken)
    {
        try
        {
            // --------  Short db read: get current platform --------
            string? platformName;
            string? platformAddress;

            await using (var scope = scopeFactory.CreateAsyncScope())
            {
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var platform = await uow.Platforms.GetByIdAsync(evt.Id, cancellationToken);

                if (platform is null)
                {
                    logger.LogError("Platform with id {PlatformId} not found for synchronization.", evt.Id);
                    return;
                }

                platformName = platform.Name;
                platformAddress = platform.Address;
            }

            if (!evt.IsOnLine)
            {
                var offlineItem = new PlatformOfflineSyncWorkItem(platformStreamManager, notifQueue, evt.Id, logger);
                await dbWorkQueue.EnqueueAsync(offlineItem, cancellationToken);
                return;
            }

            // Platform is online, fetch latest info from platform connector
            var param = new GetPlatformCommand(
                PlatformName: platformName!,
                PlatformAddress: platformAddress!
            );

            var result = await connectorFactory
                .GetConnector(evt.Type)
                .GetPlatformAsync(param, cancellationToken);

            if (!result.IsSuccess(out var platformInfo, out var error))
            {
                logger.LogError(
                    "Failed to get platform info for {Address}: {Error}",
                    platformAddress,
                    error?.Message
                );
                return;
            }

            // Enqueue DB work item to apply updates
            var workItem = new PlatformOnlineSyncWorkItem(platformStreamManager, notifQueue, platformInfo, evt.Id, logger);
            await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while synchronizing platform {Address}", evt.Address);
        }
    }
}

internal sealed class PlatformOnlineSyncWorkItem(
    IPlatformStreamManager platformStreamManager,
    INotificationQueue notificationQueue, 
    PlatformResult platformInfo, 
    Guid platformId,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            if (platform is null)
            {
                logger.LogWarning("Platform {PlatformId} not found during online sync write.", platformId);
                return;
            }

            // Apply updates from platformInfo
            platform.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networkCount: platformInfo.NetworkCount,
                volumeCount: platformInfo.VolumeCount,
                imageCount: platformInfo.ImageCount,
                memTotal: platformInfo.MemTotal,
                serverVersion: platformInfo.ServerVersion,
                agentVersion: platformInfo.AgentVersion,
                cpuCount: platformInfo.CpuCount,
                descriptor: platformInfo.Descriptor
            );

            await uow.Platforms.UpdatePlatformAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            // Notify clients
            var notificationWorkItem = new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error during PlatformOnlineSyncWorkItem for {PlatformId}", platformId);
        }
    }
}

internal sealed class PlatformOfflineSyncWorkItem(
    IPlatformStreamManager platformStreamManager, 
    INotificationQueue notificationQueue, 
    Guid PlatformId, ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await uow.Platforms.GetByIdAsync(PlatformId, cancellationToken);
            if (platform is null)
            {
                logger.LogWarning("Platform {PlatformId} not found during offline sync write.", PlatformId);
                return;
            }

            platform.PartialUpdate(platformStatus: PlatformStatus.Offline);

            await uow.Platforms.UpdatePlatformAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var notificationWorkItem = new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error during PlatformOfflineSyncWorkItem for {PlatformId}", PlatformId);
        }
    }
}

internal class PushPlatformUpdateNotificationWorkItem(IPlatformStreamManager platformStreamManager, Platform platform) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => platformStreamManager.PushPlatformUpdate(platform);
}