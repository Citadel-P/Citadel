using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Immutable;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Background job that synchronizes containers for all platforms.
/// The job ensures that new containers are added, updated containers are refreshed,
/// and stale containers are removed from the local database.
/// Synchronization occurs in two scenarios:
/// 1. Whenever a platform's status changes (e.g., from offline to online or during recovery).
/// 2. Periodically, every 6 hours, to ensure the local container state remains consistent.
/// </summary>
internal sealed class ContainerSyncJob(
    IDbWorkQueue dbWorkQueue,
    ISyncBarrier syncBarrier,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{JobName} started. Running every {H} hours.",
            nameof(ContainerSyncJob), SyncInterval.TotalHours);

        // Event-driven sync starts immediately
        var eventDrivenTask = RunEventDrivenSync(cancellationToken);

        // Periodic sync starts in parallel
        var periodicTask = RunPeriodicSync(cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    /// <summary>
    /// Reacts to platform health events and syncs containers for platforms as they change state.
    /// </summary>
    private async Task RunEventDrivenSync(CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            try
            {
                await EnqueueSyncForPlatform(syncBarrier, platformEvent, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex,
                    "Unhandled error while scheduling container sync for {Address}",
                    platformEvent.Address);
            }
        }
    }

    /// <summary>
    /// Periodically syncs all platforms.
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
                logger.LogError(ex, "Error during periodic container synchronization.");
            }

            try
            {
                await Task.Delay(SyncInterval, cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                // normal shutdown
                break;
            }
        }
    }

    /// <summary>
    /// Fetches all platforms from the cache and schedules sync for each.
    /// </summary>
    private async Task SyncAllPlatforms(CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntries(out var platforms, out _))
        {
            logger.LogDebug("No platforms found in cache for periodic sync.");
            return;
        }

        foreach (var platform in platforms)
        {
            if (cancellationToken.IsCancellationRequested)
                break;

            // Wait until ImageSyncJob has synced images at least once
            await syncBarrier.WaitForAsync<ImageSyncJob>(platform.Id);

            try
            {
                var platformEvent = new PlatformHealth(
                    Id: platform.Id,
                    Type: platform.ConnectorType,
                    Address: platform.Address,
                    IsOnLine: true);

                await EnqueueSyncForPlatform(syncBarrier, platformEvent, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex,
                    "Error scheduling container sync for platform {PlatformId}",
                    platform.Id);
            }
        }
    }

    /// <summary>
    /// Entry point for both event-driven and periodic sync:
    /// - If platform is online → call Docker API, then enqueue DB work.
    /// - If offline → enqueue DB work that marks containers offline.
    /// </summary>
    private async Task EnqueueSyncForPlatform(
        ISyncBarrier syncBarrier,
        PlatformHealth platformEvent,
        CancellationToken cancellationToken)
    {
        if (platformEvent.IsOnLine)
        {
            logger.LogInformation(
                "Platform {PlatformId} is online. Scheduling container sync from {Address}...",
                platformEvent.Id, platformEvent.Address);

            var command = new ContainerFilterCommand(
                PlatformAddress: platformEvent.Address,
                All: true);

            var result = await connectorFactory
                .GetConnector(platformEvent.Type)
                .ListContainersAsync(command, cancellationToken: cancellationToken);

            if (!result.IsSuccess(out var freshContainers, out var error))
            {
                logger.LogError(
                    "Error retrieving container list for platform {PlatformId} at {Address}: {Error}",
                    platformEvent.Id, platformEvent.Address, error);
                return;
            }

            var workItem = new SyncOnlinePlatformContainersWorkItem(
                platformEvent,
                notificationQueue,
                freshContainers,
                platformContainerCache,
                containerStreamManager,
                dbWorkQueue,
                deploymentStreamManager,
                stackStreamManager,
                logger);

            await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
        }
        else
        {
            logger.LogInformation(
                "Platform {PlatformId} is offline. Scheduling containers offline update.",
                platformEvent.Id);

            var workItem = new SyncOfflinePlatformContainersWorkItem(
                platformEvent.Id,
                notificationQueue,
                platformContainerCache,
                containerStreamManager,
                dbWorkQueue,
                deploymentStreamManager,
                stackStreamManager,
                logger);

            await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
        }
    }
}

internal sealed class SyncOnlinePlatformContainersWorkItem(
    PlatformHealth platformEvent,
    INotificationQueue notificationQueue,
    IReadOnlyDictionary<string, DockerContainer> freshContainers,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IDbWorkQueue dbWorkQueue,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var images = await uow.Images.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
            var containers = await uow.Containers.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);

            var existingContainersInDb = containers.ToDictionary(
                c => c.DockerContainerId,
                c => c,
                StringComparer.OrdinalIgnoreCase);

            var currentActiveContainers = new List<Container>();

            // Map fresh containers
            foreach (var freshContainer in freshContainers.Values)
            {
                var imageId = images.FirstOrDefault(i => i.DockerImageId == freshContainer.ImageId)?.Id;

                if (existingContainersInDb.TryGetValue(freshContainer.Id, out var existingDbContainer))
                {
                    existingDbContainer.PartialUpdate(
                        name: freshContainer.Name,
                        imageId: imageId,
                        dockerImageId: freshContainer.ImageId,
                        state: freshContainer.State,
                        dockerStack: freshContainer.Stack,
                        created: freshContainer.Created,
                        ports: freshContainer.Ports,
                        stackId: freshContainer.StackId);

                    currentActiveContainers.Add(existingDbContainer);
                }
                else
                {
                    var container = freshContainer.Map(platformEvent.Id, imageId);
                    currentActiveContainers.Add(container);
                }
            }

            // Upsert current active containers
            await uow.Containers.BulkUpsertAsync(currentActiveContainers, cancellationToken);

            // Remove stale
            var freshIds = freshContainers.Keys.ToHashSet(StringComparer.OrdinalIgnoreCase);
            var staleContainers = existingContainersInDb.Values
                .Where(c => !freshIds.Contains(c.DockerContainerId))
                .ToArray();

            if (staleContainers.Length > 0)
            {
                logger.LogInformation(
                    "Removing {Count} stale containers for platform {PlatformId}.",
                    staleContainers.Length, platformEvent.Id);

                await uow.Containers.DeleteAsync(
                    staleContainers.Select(s => s.Id),
                    cancellationToken);
            }

            await uow.CommitAsync(cancellationToken);

            // Refresh cache
            var cacheEntry = new PlatformCacheEntry(
                Id: platformEvent.Id,
                Address: platformEvent.Address,
                ConnectorType: platformEvent.Type,
                Containers: currentActiveContainers.ToImmutableDictionary(
                    c => c.DockerContainerId,
                    c => c.Id,
                    StringComparer.OrdinalIgnoreCase));

            platformContainerCache.ReplacePlatformContainers(platformEvent.Id, cacheEntry);

            // Notify clients
            var notificationWorkItem = new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                currentActiveContainers,
                platformEvent.Id);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

            await ScheduleDeploymentSync(platformEvent.Id, true, cancellationToken);
            await ScheduleStackSync(platformEvent.Id, true, cancellationToken);

            logger.LogInformation(
                "Synchronized {Count} containers for platform {PlatformId}.",
                currentActiveContainers.Count, platformEvent.Id);
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Error while syncing containers for platform {PlatformId} at {Address}",
                platformEvent.Id, platformEvent.Address);
        }
    }

    internal ValueTask ScheduleDeploymentSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(
                new DeploymentSyncWorkItem(
                    deploymentStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline),
                ct);

    internal ValueTask ScheduleStackSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(
                new StackSyncWorkItem(
                    stackStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline),
                ct);
}

internal sealed class SyncOfflinePlatformContainersWorkItem(
    Guid platformId,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IDbWorkQueue dbWorkQueue,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var offlineContainers = await uow.Containers.GetByPlatformIdAsync(platformId, cancellationToken);

            await uow.Containers.UpdateContainersStateAsync(
                offlineContainers.Select(c => c.Id),
                ContainerStateStatus.Offline,
                cancellationToken);

            foreach (var container in offlineContainers)
            {
                container.PartialUpdate(state: ContainerStateStatus.Offline);
            }

            await uow.CommitAsync(cancellationToken);

            // Evict platform from cache
            platformContainerCache.EvictPlatform(platformId);

            // Notify clients that containers went offline
            var notificationWorkItem = new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                offlineContainers,
                platformId);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

            await ScheduleDeploymentSync(platformId, false, cancellationToken);
            await ScheduleStackSync(platformId, false, cancellationToken);

            logger.LogInformation(
                "Marked {Count} containers as offline for platform {PlatformId}.",
                offlineContainers.Count(), platformId);
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Error while marking containers offline for platform {PlatformId}.",
                platformId);
        }
    }

    internal ValueTask ScheduleDeploymentSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(
                new DeploymentSyncWorkItem(
                    deploymentStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline),
                ct);

    internal ValueTask ScheduleStackSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(
                new StackSyncWorkItem(
                    stackStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline),
                ct);
}

internal class SendContainersInfoNotificationWorkItem(IContainerStreamManager containerStreamManager, IEnumerable<Container> containers, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => containerStreamManager.SendContainersInfo(platformId, containers);
}
