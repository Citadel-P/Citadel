using Application.Features.Deployments.Notifications;
using Application.Features.Platforms;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
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
    IActivityStreamManager activityStreamManager,
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
        logger.LogInformation("{JobName} started. Running every {H} hours.", nameof(PlatformSyncJob), SyncInterval.TotalHours);

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
            if (evt.IsValidated)
                continue;

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
                logger.LogError(ex, "Error during periodic {JobName} synchronization.", nameof(PlatformSyncJob));
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
    internal async Task SyncPlatform(PlatformHealth evt, CancellationToken cancellationToken)
    {
        try
        {
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
                var offlineItem = new PlatformOfflineSyncWorkItem(
                    platformStreamManager,
                    activityStreamManager,
                    notifQueue,
                    evt.Id,
                    logger);
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
            var workItem = new PlatformOnlineSyncWorkItem(
                platformStreamManager,
                activityStreamManager,
                notifQueue,
                platformHealthBroadCaster,
                platformContainerCache,
                platformInfo,
                evt.Id,
                logger);
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
    IActivityStreamManager activityStreamManager,
    INotificationQueue notificationQueue,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IPlatformContainerCache platformContainerCache,
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

            var previousStatus = platform.Status;
            var updatedDescriptor = platformInfo.Descriptor;
            string? clusterId = null;
            if (platformInfo.Descriptor is null
                || platformInfo.Descriptor.Type != platform.PlatformDescriptor.Type)
            {
                logger.LogError(
                    "Platform {PlatformId} synchronization was rejected because its platform type changed",
                    platformId);
                await RejectAsync(platform, uow, cancellationToken);
                return;
            }

            if (platform.PlatformDescriptor is DockerSwarmPlatformDescriptor currentSwarm)
            {
                if (platformInfo.Descriptor is not DockerSwarmPlatformDescriptor reportedSwarm
                    || !reportedSwarm.ControlAvailable
                    || string.IsNullOrWhiteSpace(reportedSwarm.ClusterId))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because the endpoint is not an active Swarm manager",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                clusterId = reportedSwarm.ClusterId.Trim();
                if (!string.IsNullOrWhiteSpace(platform.ClusterId)
                    && !string.Equals(platform.ClusterId, clusterId, StringComparison.Ordinal))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because its Swarm cluster identity changed",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                if (!string.IsNullOrWhiteSpace(currentSwarm.NodeID)
                    && !string.Equals(currentSwarm.NodeID, reportedSwarm.NodeID, StringComparison.Ordinal))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because its pinned Swarm manager Node identity changed",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                if (!string.IsNullOrWhiteSpace(currentSwarm.DaemonId)
                    && !string.Equals(currentSwarm.DaemonId, reportedSwarm.DaemonId, StringComparison.Ordinal))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because its pinned Swarm manager Docker daemon identity changed",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                if (string.IsNullOrWhiteSpace(platform.ClusterId))
                {
                    var existingCluster = await uow.Platforms.GetByClusterIdAsync(
                        clusterId,
                        platformId,
                        cancellationToken);
                    if (existingCluster is not null)
                    {
                        logger.LogError(
                            "Platform {PlatformId} synchronization was rejected because Swarm cluster {ClusterId} belongs to platform {ExistingPlatformId}",
                            platformId,
                            clusterId,
                            existingCluster.Id);
                        await RejectAsync(platform, uow, cancellationToken);
                        return;
                    }
                }

                updatedDescriptor = reportedSwarm with { ClusterId = clusterId };
            }

            if (platform.PlatformDescriptor is DockerPlatformDescriptor currentDescriptor)
            {
                if (platformInfo.Descriptor is not DockerPlatformDescriptor reportedDescriptor
                    || string.IsNullOrWhiteSpace(reportedDescriptor.DaemonId))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because Docker did not report a daemon id",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                var currentDaemonId =
                    currentDescriptor.DaemonId?.Trim() ?? string.Empty;
                var reportedDaemonId = reportedDescriptor.DaemonId.Trim();
                if (currentDaemonId.Length > 0
                    && !string.Equals(
                        currentDaemonId,
                        reportedDaemonId,
                        StringComparison.Ordinal))
                {
                    logger.LogError(
                        "Platform {PlatformId} synchronization was rejected because its Docker daemon identity changed",
                        platformId);
                    await RejectAsync(platform, uow, cancellationToken);
                    return;
                }

                if (currentDaemonId.Length == 0)
                {
                    var existingPlatform =
                        await uow.Platforms.GetByDaemonIdAsync(
                            reportedDaemonId,
                            platformId,
                            cancellationToken);
                    if (existingPlatform is not null)
                    {
                        logger.LogError(
                            "Platform {PlatformId} synchronization was rejected because Docker daemon {DaemonId} belongs to platform {ExistingPlatformId}",
                            platformId,
                            reportedDaemonId,
                            existingPlatform.Id);
                        await RejectAsync(platform, uow, cancellationToken);
                        return;
                    }
                }

                updatedDescriptor =
                    reportedDescriptor with { DaemonId = reportedDaemonId };
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
                descriptor: updatedDescriptor,
                clusterId: clusterId
            );

            var activity = previousStatus == PlatformStatus.Online
                ? null
                : PlatformActivity.Connected(platform, previousStatus, Constants.SystemId);
            if (activity is not null)
            {
                await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
            }

            await uow.Platforms.UpdateAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            await platformHealthBroadCaster.PublishAsync(
                new PlatformHealth(
                    platform.Id,
                    platform.Address,
                    platform.ConnectorType,
                    IsOnLine: true,
                    IsValidated: true),
                cancellationToken);

            // Notify clients
            var notificationWorkItem = new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);
            if (activity is not null)
            {
                await notificationQueue.EnqueueAsync(
                    new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(uow, cancellationToken)),
                    cancellationToken);
            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error during PlatformOnlineSyncWorkItem for {PlatformId}", platformId);
        }
    }

    private async Task RejectAsync(
        Platform platform,
        IUnitOfWork uow,
        CancellationToken cancellationToken)
    {
        platformContainerCache.EvictPlatform(platform.Id);
        await platformHealthBroadCaster.PublishAsync(
            new PlatformHealth(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                IsOnLine: false,
                IsValidated: true),
            cancellationToken);

        var previousStatus = platform.Status;
        if (previousStatus == PlatformStatus.Offline)
            return;

        platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
        var activity = PlatformActivity.Disconnected(platform, previousStatus, Constants.SystemId);
        await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await uow.Platforms.UpdateAsync(platform, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform),
            cancellationToken);
        await notificationQueue.EnqueueAsync(
            new ActivityNotificationWorkItem(
                activityStreamManager,
                await activity.AssignActor(uow, cancellationToken)),
            cancellationToken);
    }
}

internal sealed class PlatformOfflineSyncWorkItem(
    IPlatformStreamManager platformStreamManager,
    IActivityStreamManager activityStreamManager,
    INotificationQueue notificationQueue,
    Guid PlatformId,
    ILogger logger) : IDbWorkItem
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

            var previousStatus = platform.Status;
            platform.PartialUpdate(platformStatus: PlatformStatus.Offline);

            var activity = previousStatus == PlatformStatus.Offline
                ? null
                : PlatformActivity.Disconnected(platform, previousStatus, Constants.SystemId);
            if (activity is not null)
            {
                await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
            }

            await uow.Platforms.UpdateAsync(platform, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var notificationWorkItem = new PushPlatformUpdateNotificationWorkItem(platformStreamManager, platform);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);
            if (activity is not null)
            {
                await notificationQueue.EnqueueAsync(
                    new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(uow, cancellationToken)),
                    cancellationToken);
            }
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
