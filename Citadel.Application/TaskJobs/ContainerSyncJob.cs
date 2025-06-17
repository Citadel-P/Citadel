using System.Threading.Channels;
using Application.Services;
using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Syncing full containers state when platform status change.
/// </summary>
internal class ContainerSyncJob(
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
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

    private async Task SyncContainersForPlatform(PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

        Container[] syncedContainers;

        if (platformEvent.IsOnLine)
        {
            logger.LogInformation("Platform {PlatformId} is online. Syncing containers from {Address}...", platformEvent.Id, platformEvent.Address);
            syncedContainers = await SyncOnlinePlatformContainers(uow, platformEvent, cancellationToken);
            var cacheEntry = new PlatformCacheEntry
            (
                Type: platformEvent.Type,
                PlatformAddress: platformEvent.Address,
                Containers: syncedContainers.ToDictionary(c => c.ContainerId, c => c.Id)
            );
            platformContainerCache.ReplacePlatformContainers(platformEvent.Id, cacheEntry);
        }
        else
        {
            logger.LogInformation("Platform {PlatformId} is offline. Updating container states.", platformEvent.Id);
            syncedContainers = await SyncOfflinePlatformContainers(uow, platformEvent.Id, cancellationToken);
            platformContainerCache.EvictPlatform(platformEvent.Id);
        }

        // Notify clients and update the cache
        await containerHub.SendContainersInfo(platformEvent.Id, syncedContainers);

        logger.LogInformation("Successfully synchronized {Count} containers for platform {PlatformId}.", syncedContainers.Length, platformEvent.Id);
    }

    private async Task<Container[]> SyncOnlinePlatformContainers(IUnitOfWork unitOfWork, PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand
        (
            PlatformAddress: platformEvent.Address,
            PlatformId: platformEvent.Id,
            All: true
        );

        var result = await connectorFactory.GetConnector(platformEvent.Type).ListContainersAsync(command, cancellationToken: cancellationToken);
        if (!result.IsSuccess(out var freshContainers, out var error))
        {
            logger.LogError("Failed to list containers for platform {PlatformId} at {Address}: {Error}", platformEvent.Id, platformEvent.Address, error);
            return [];
        }


        var existingContainersInDb = await unitOfWork.Containers
            .Query().Where(c => c.PlatformId == platformEvent.Id)
            .ToDictionaryAsync(c => c.ContainerId, c => c, cancellationToken);

        // This list will hold the containers that are currently active and should be cached
        var currentActiveContainers = new List<Container>();

        foreach (var freshContainer in freshContainers.Values.ToList())
        {
            if (existingContainersInDb.TryGetValue(freshContainer.ContainerId, out var existingDbContainer))
            {
                // Update existing container in DB context
                existingDbContainer.PartialUpdate(
                    name: freshContainer.Name,
                    image: freshContainer.Image,
                    state: freshContainer.State,
                    stack: freshContainer.Stack,
                    created: freshContainer.Created,
                    ports: freshContainer.Ports
                );
                currentActiveContainers.Add(existingDbContainer); 
            }
            else
            {
                // Add new container to DB
                unitOfWork.Containers.Add(freshContainer);
                currentActiveContainers.Add(freshContainer);
            }
        }

        // Remove stale containers (those in DB but not in freshContainers)
        var staleContainers = existingContainersInDb.Values
            .Where(c => !freshContainers.ContainsKey(c.ContainerId))
            .ToArray();

        if (staleContainers.Length > 0)
        {
            logger.LogInformation("Removing {Count} stale containers for platform {PlatformId}.", staleContainers.Length, platformEvent.Id);
            unitOfWork.Containers.RemoveRange(staleContainers);
        }

        await unitOfWork.SaveChangesAsync(cancellationToken);

        return [.. currentActiveContainers];
    }
    private static async Task<Container[]> SyncOfflinePlatformContainers(IUnitOfWork unitOfWork, Guid platformId, CancellationToken cancellationToken)
    {
        var offlineContainers = await unitOfWork.Containers
            .Query().Where(c => c.PlatformId == platformId)
            .ToArrayAsync(cancellationToken);

        foreach (var container in offlineContainers)
        {
            container.PartialUpdate(state: ContainerStateStatus.Offline);
        }

        await unitOfWork.SaveChangesAsync(cancellationToken);
        return offlineContainers;
    }
}