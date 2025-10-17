using System.Threading.Channels;
using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Background job that synchronizes containers for all platforms.
/// The job ensures that new containers are added, updated containers are refreshed, and stale containers are removed from the local database.
/// Synchronization occurs in two scenarios:
/// 1. Whenever a platform's status changes (e.g., from offline to online or during recovery).
/// 2. Periodically, every 12 hours, to ensure the local container state remains consistent with the platform state.
/// </summary>
internal class ContainerSyncJob(
    ISyncBarrier syncBarrier,
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IContainerStreamManager containerStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(12);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{ContainerSyncJob} started. Running every {Hours} hours.", nameof(ContainerSyncJob), SyncInterval.TotalHours);

        // Wait until ImageSyncJob has synced images at least once
        await syncBarrier.WaitForAsync<ImageSyncJob>(cancellationToken);

        // Event-driven sync starts immediately
        var eventDrivenTask = RunEventDrivenSync(cancellationToken);

        // Periodic sync starts with jitter
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

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
                await SyncContainersForPlatform(platformEvent, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Unhandled error while synchronizing containers for {Address}", platformEvent.Address);
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
                logger.LogError(ex, "Error during periodic container synchronization.");
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    /// <summary>
    /// Fetches all platforms from the database and syncs them.
    /// </summary>
    private async Task SyncAllPlatforms(CancellationToken cancellationToken)
    {
        if (platformContainerCache.TryGetCacheEntries( out var platforms, out var _))
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

                    await SyncContainersForPlatform(platformEvent, cancellationToken);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error syncing containers for platform {PlatformId}", platform.Id);
                }
            }
        }
    }

    /// <summary>
    /// Core logic to synchronize containers for a single platform.
    /// </summary>
    private async Task SyncContainersForPlatform(PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        IEnumerable<Container> syncedContainers;

        if (platformEvent.IsOnLine)
        {
            logger.LogInformation("Platform {PlatformId} is online. Synchronizing containers from {Address}...", platformEvent.Id, platformEvent.Address);
            syncedContainers = await SyncOnlinePlatformContainers(uow, platformEvent, cancellationToken);
            var cacheEntry = new PlatformCacheEntry
            (
                Id: platformEvent.Id,
                Address: platformEvent.Address,
                ConnectorType: platformEvent.Type,
                Containers: syncedContainers.ToDictionary(c => c.DockerContainerId, c => c.Id)
            );
            platformContainerCache.ReplacePlatformContainers(platformEvent.Id, cacheEntry);
        }
        else
        {
            logger.LogInformation("Platform {PlatformId} is offline. Updating container states.", platformEvent.Id);
            syncedContainers = await SyncOfflinePlatformContainers(uow, platformEvent.Id, cancellationToken);
            platformContainerCache.EvictPlatform(platformEvent.Id);
        }

        // Notify clients
        await containerStreamManager.SendContainersInfo(platformEvent.Id, syncedContainers);

        logger.LogInformation("Synchronized {Count} containers for platform {PlatformId}.", syncedContainers.Count(), platformEvent.Id);
    }

    private async Task<List<Container>> SyncOnlinePlatformContainers(
    IUnitOfWork unitOfWork,
    PlatformHealth platformEvent,
    CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand(
            PlatformAddress: platformEvent.Address,
            All: true
        );

        var result = await connectorFactory
            .GetConnector(platformEvent.Type)
            .ListContainersAsync(command, cancellationToken: cancellationToken);

        if (!result.IsSuccess(out var freshContainers, out var error))
        {
            logger.LogError(
                "An error occurred while retrieving the container list for platform {PlatformId} at {Address}: {Error}",
                platformEvent.Id, platformEvent.Address, error
            );
            return [];
        }

        var images = await unitOfWork.Images.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
        var containers = await unitOfWork.Containers.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
        var existingContainersInDb = containers.ToDictionary(c => c.DockerContainerId, c => c, StringComparer.OrdinalIgnoreCase);

        var currentActiveContainers = new List<Container>();

        foreach (var freshContainer in freshContainers.Values)
        {
            var imageId = images.FirstOrDefault(i => i.DockerImageId == freshContainer.ImageId)?.Id;
            if (existingContainersInDb.TryGetValue(freshContainer.ContainerId, out var existingDbContainer))
            {
                existingDbContainer.PartialUpdate(
                    name: freshContainer.Name,
                    imageId: imageId,
                    dockerImageId: freshContainer.ImageId,
                    state: freshContainer.State,
                    stack: freshContainer.Stack,
                    created: freshContainer.Created,
                    ports: freshContainer.Ports
                );
                currentActiveContainers.Add(existingDbContainer);
            }
            else
            {
                var container = freshContainer.Map(platformEvent.Id, imageId);
                currentActiveContainers.Add(container);
            }
        }

        await unitOfWork.Containers.BulkUpsertAsync(currentActiveContainers, cancellationToken);

        // Remove stale
        var freshIds = freshContainers.Keys.ToHashSet(StringComparer.OrdinalIgnoreCase);
        var staleContainers = existingContainersInDb.Values
            .Where(c => !freshIds.Contains(c.DockerContainerId))
            .ToArray();

        if (staleContainers.Length > 0)
        {
            logger.LogInformation(
                "Removing {Count} stale containers for platform {PlatformId}.",
                staleContainers.Length, platformEvent.Id
            );
            await unitOfWork.Containers.DeleteAsync(staleContainers.Select(s => s.Id), cancellationToken);
        }

        await unitOfWork.CommitAsync();
        return currentActiveContainers;
    }

    private static async Task<IEnumerable<Container>> SyncOfflinePlatformContainers(IUnitOfWork unitOfWork, Guid platformId, CancellationToken cancellationToken)
    {
        var offlineContainers = await unitOfWork.Containers.GetByPlatformIdAsync(platformId, cancellationToken);
        await unitOfWork.Containers.UpdateContainersStateAsync(offlineContainers.Select(c => c.Id), ContainerStateStatus.Offline, cancellationToken);
        foreach (var container in offlineContainers)
        {
            container.PartialUpdate(state: ContainerStateStatus.Offline);
        }

        await unitOfWork.CommitAsync();
        return offlineContainers;
    }
}