using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
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
    SwarmTaskContainerPruner swarmTaskContainerPruner,
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
                if (platformEvent.IsOnLine && !platformEvent.IsValidated)
                    continue;

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
            await syncBarrier.WaitForAsync<ImageSyncJob>(platform.Id, cancellationToken);

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
            var snapshotStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var cacheMutationVersion = platformContainerCache.GetMutationVersion(platformEvent.Id);
            logger.LogInformation(
                "Platform {PlatformId} is online. Scheduling container sync from {Address}...",
                platformEvent.Id, platformEvent.Address);

            var command = new ContainerFilterCommand(
                PlatformAddress: platformEvent.Address,
                All: true);

            var connector = connectorFactory.GetConnector(platformEvent.Type);
            var result = await connector
                .ListContainersAsync(command, cancellationToken: cancellationToken);

            if (!result.IsSuccess(out var freshContainers, out var error))
            {
                logger.LogError(
                    "Error retrieving container list for platform {PlatformId} at {Address}: {Error}",
                    platformEvent.Id, platformEvent.Address, error);
                return;
            }

            await swarmTaskContainerPruner.PruneAsync(
                platformEvent,
                connector,
                freshContainers.Values,
                cancellationToken);

            var workItem = new SyncOnlinePlatformContainersWorkItem(
                platformEvent,
                notificationQueue,
                freshContainers,
                platformContainerCache,
                containerStreamManager,
                deploymentStreamManager,
                stackStreamManager,
                snapshotStartedAt,
                cacheMutationVersion,
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
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    long snapshotStartedAt,
    long cacheMutationVersion,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            // Container daemon events and other syncs mutate this version only after their
            // database transaction commits. Discard a list captured before such a mutation;
            // otherwise a stale list can recreate a container that a newer event deleted.
            if (platformContainerCache.GetMutationVersion(platformEvent.Id) != cacheMutationVersion)
                return;

            var images = await uow.Images.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
            var platform = await uow.Platforms.GetByIdAsync(platformEvent.Id, cancellationToken);
            var containers = (await uow.Containers.GetByPlatformIdAsync(platformEvent.Id, cancellationToken)).ToArray();
            var managerNodeId = (platform?.PlatformDescriptor as DockerSwarmPlatformDescriptor)?.NodeID;
            var sourceContainers = managerNodeId is null
                ? containers
                : containers.Where(container =>
                    container.DockerNodeId is null
                    || string.Equals(container.DockerNodeId, managerNodeId, StringComparison.Ordinal)).ToArray();
            var imageIds = images.ToDictionary(
                image => image.DockerImageId,
                image => image.Id,
                StringComparer.OrdinalIgnoreCase);

            var existingContainersInDb = sourceContainers.ToDictionary(
                c => c.DockerContainerId,
                c => c,
                StringComparer.OrdinalIgnoreCase);

            var currentActiveContainers = new List<Container>();
            var containersToUpsert = new List<Container>();
            var currentDockerContainers = freshContainers.Values
                .Where(static container => !container.IsHistoricalSwarmTask())
                .ToArray();

            // Map fresh containers
            foreach (var freshContainer in currentDockerContainers)
            {
                Guid? imageId = imageIds.TryGetValue(freshContainer.ImageId, out var resolvedImageId)
                    ? resolvedImageId
                    : null;

                if (existingContainersInDb.TryGetValue(freshContainer.Id, out var existingDbContainer))
                {
                    if (existingDbContainer.Updated >= snapshotStartedAt)
                    {
                        currentActiveContainers.Add(existingDbContainer);
                        continue;
                    }

                    existingDbContainer.PartialUpdate(
                        name: freshContainer.Name,
                        imageId: imageId,
                        dockerImageId: freshContainer.ImageId,
                        state: freshContainer.State,
                        dockerStack: freshContainer.Stack,
                        created: freshContainer.Created,
                        ports: freshContainer.Ports,
                        stackId: existingDbContainer.StackId ?? freshContainer.StackId,
                        isSystem: freshContainer.IsSystem,
                        systemRole: freshContainer.SystemRole,
                        hasCitadelOwnershipLabels: freshContainer.HasCitadelOwnershipLabels,
                        isSwarmTask: freshContainer.IsSwarmTask);
                    if (managerNodeId is not null)
                        existingDbContainer.ObserveOnNode(managerNodeId, snapshotStartedAt);

                    currentActiveContainers.Add(existingDbContainer);
                    containersToUpsert.Add(existingDbContainer);
                }
                else
                {
                    var container = freshContainer.Map(
                        platformEvent.Id,
                        imageId,
                        managerNodeId,
                        managerNodeId is null ? null : snapshotStartedAt);
                    currentActiveContainers.Add(container);
                    containersToUpsert.Add(container);
                }
            }

            // Upsert current active containers
            if (containersToUpsert.Count > 0)
                await uow.Containers.BulkUpsertAsync(containersToUpsert, cancellationToken);

            // Remove stale
            var freshIds = currentDockerContainers
                .Select(static container => container.Id)
                .ToHashSet(StringComparer.OrdinalIgnoreCase);
            var staleContainers = existingContainersInDb.Values
                .Where(c => !freshIds.Contains(c.DockerContainerId) && c.Updated < snapshotStartedAt)
                .ToArray();

            currentActiveContainers.AddRange(existingContainersInDb.Values.Where(
                container => !freshIds.Contains(container.DockerContainerId) &&
                             container.Updated >= snapshotStartedAt));

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

            var allCurrentContainers = (await uow.Containers.GetContainersInfoAsync(
                platformEvent.Id,
                cancellationToken))?.ToArray() ?? [];

            // Refresh cache
            var cacheEntry = new PlatformCacheEntry(
                Id: platformEvent.Id,
                Address: platformEvent.Address,
                ConnectorType: platformEvent.Type,
                Containers: allCurrentContainers.ToImmutableDictionary(
                    c => c.DockerContainerId,
                    c => c.Id,
                    StringComparer.OrdinalIgnoreCase));

            platformContainerCache.ReplacePlatformContainers(platformEvent.Id, cacheEntry);

            // Notify clients
            var notificationWorkItem = new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                allCurrentContainers,
                platformEvent.Id);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

            await ContainerDependentResourceSynchronizer.SynchronizeAsync(
                uow,
                deploymentStreamManager,
                stackStreamManager,
                notificationQueue,
                platformEvent.Id,
                true,
                logger,
                cancellationToken);

            logger.LogInformation(
                "Synchronized {Count} containers for platform {PlatformId}.",
                allCurrentContainers.Length, platformEvent.Id);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Error while syncing containers for platform {PlatformId} at {Address}",
                platformEvent.Id, platformEvent.Address);
        }
    }

}

internal sealed class SyncOfflinePlatformContainersWorkItem(
    Guid platformId,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            var allContainers = (await uow.Containers.GetByPlatformIdAsync(platformId, cancellationToken)).ToArray();
            var managerNodeId = (platform?.PlatformDescriptor as DockerSwarmPlatformDescriptor)?.NodeID;
            var offlineContainers = managerNodeId is null
                ? allContainers
                : allContainers.Where(container =>
                    container.DockerNodeId is null
                    || string.Equals(container.DockerNodeId, managerNodeId, StringComparison.Ordinal)).ToArray();

            await uow.Containers.UpdateContainersStateAsync(
                offlineContainers.Select(c => c.Id),
                ContainerStateStatus.Offline,
                cancellationToken);

            foreach (var container in offlineContainers)
            {
                container.PartialUpdate(state: ContainerStateStatus.Offline);
            }

            if (managerNodeId is not null)
            {
                await uow.Containers.MarkNodeProjectionStaleAsync(
                    platformId,
                    managerNodeId,
                    "Manager data source is offline.",
                    DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    cancellationToken);
            }

            await uow.CommitAsync(cancellationToken);

            var currentContainers = (await uow.Containers.GetContainersInfoAsync(platformId, cancellationToken))?.ToArray() ?? [];
            if (managerNodeId is not null && platform is not null)
            {
                platformContainerCache.ReplacePlatformContainers(
                    platformId,
                    new PlatformCacheEntry(
                        platformId,
                        platform.Address,
                        platform.ConnectorType,
                        currentContainers.ToImmutableDictionary(
                            container => container.DockerContainerId,
                            container => container.Id,
                            StringComparer.OrdinalIgnoreCase)));
            }
            else
            {
                platformContainerCache.EvictPlatform(platformId);
            }

            // Notify clients that containers went offline
            var notificationWorkItem = new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                currentContainers,
                platformId);
            await notificationQueue.EnqueueAsync(notificationWorkItem, cancellationToken);

            await ContainerDependentResourceSynchronizer.SynchronizeAsync(
                uow,
                deploymentStreamManager,
                stackStreamManager,
                notificationQueue,
                platformId,
                false,
                logger,
                cancellationToken);

            logger.LogInformation(
                "Marked {Count} containers as offline for platform {PlatformId}.",
                offlineContainers.Length, platformId);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Error while marking containers offline for platform {PlatformId}.",
                platformId);
        }
    }

}

internal static class ContainerDependentResourceSynchronizer
{
    internal static async Task SynchronizeAsync(
        IUnitOfWork uow,
        IDeploymentStreamManager deploymentStreamManager,
        IStackStreamManager stackStreamManager,
        INotificationQueue notificationQueue,
        Guid platformId,
        bool isOnline,
        ILogger logger,
        CancellationToken cancellationToken)
    {
        await ExecuteSafelyAsync(
            "deployments",
            () => new DeploymentSyncWorkItem(
                    deploymentStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline)
                .ExecuteAsync(uow, cancellationToken));

        await ExecuteSafelyAsync(
            "stacks",
            () => new StackSyncWorkItem(
                    stackStreamManager,
                    notificationQueue,
                    platformId,
                    isOnline)
                .ExecuteAsync(uow, cancellationToken));

        async Task ExecuteSafelyAsync(string resourceType, Func<Task> synchronize)
        {
            try
            {
                await synchronize();
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                await uow.RollbackAsync();
                logger.LogError(
                    ex,
                    "Failed to synchronize dependent {ResourceType} for platform {PlatformId}.",
                    resourceType,
                    platformId);
            }
        }
    }
}

internal class SendContainersInfoNotificationWorkItem(IContainerStreamManager containerStreamManager, IEnumerable<Container> containers, Guid platformId) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => containerStreamManager.SendContainersInfo(platformId, containers);
}
