using System.Threading.Channels;
using Application.Mappers;
using Application.Services;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
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
    IContainersStreamManager containerStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    ILogger<ContainerSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();

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

        IEnumerable<Container> syncedContainers;

        if (platformEvent.IsOnLine)
        {
            logger.LogInformation("Platform {PlatformId} is online. Synchronizing containers from {Address}...", platformEvent.Id, platformEvent.Address);
            syncedContainers = await SyncOnlinePlatformContainers(uow, platformEvent, cancellationToken);
            var cacheEntry = new PlatformCacheEntry
            (
                Address: platformEvent.Address,
                ConnectorType: platformEvent.Type,
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
        await containerStreamManager.SendContainersInfo(platformEvent.Id, syncedContainers);

        logger.LogInformation("Synchronized {Count} containers for platform ID {PlatformId}.", syncedContainers.Count(), platformEvent.Id);
    }

    private async Task<List<Container>> SyncOnlinePlatformContainers(IUnitOfWork unitOfWork, PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand
        (
            PlatformAddress: platformEvent.Address,
            All: true
        );

        var result = await connectorFactory.GetConnector(platformEvent.Type).ListContainersAsync(command, cancellationToken: cancellationToken);
        if (!result.IsSuccess(out var freshContainers, out var error))
        {
            logger.LogError("An error occurred while retrieving the container list for platform ID {PlatformId} at address {Address}: {Error}", platformEvent.Id, platformEvent.Address, error); 
            return [];
        }

        var containers = await unitOfWork.Containers.GetByPlatformIdAsync(platformEvent.Id, cancellationToken);
        if (!containers.Any()) 
        {
            return [];
        }

        // For fast lookups
        var existingContainersInDb = containers.ToDictionary(c => c.ContainerId, c => c, StringComparer.OrdinalIgnoreCase);
        
        // This list will hold the containers that are currently active and should be cached
        var currentActiveContainers = new List<Container>();

        foreach (var freshContainer in freshContainers.Values)
        {
            if (existingContainersInDb.TryGetValue(freshContainer.ContainerId, out var existingDbContainer))
            {
                // Update existing container
                existingDbContainer.PartialUpdate(
                    name: freshContainer.Name,
                    image: freshContainer.Image,
                    state: freshContainer.State,
                    stack: freshContainer.Stack,
                    created: freshContainer.Created,
                    ports: freshContainer.Ports
                );
                currentActiveContainers.Add(existingDbContainer);
                // Todo bulk update 
                await unitOfWork.Containers.UpdateContainerAsync(existingDbContainer, cancellationToken);
            }
            else
            {
                // Add new container to DB
                var container = freshContainer.Map(platformEvent.Id);
                await unitOfWork.Containers.AddAsync(container, cancellationToken);
                currentActiveContainers.Add(container);
            }
        }

        // Remove stale containers (those in DB but not in freshContainers)
        var staleContainers = existingContainersInDb.Values
            .Where(c => !freshContainers.ContainsKey(c.ContainerId))
            .ToArray();

        if (staleContainers.Length > 0)
        {
            logger.LogInformation("Removing {Count} stale containers for platform {PlatformId}.", staleContainers.Length, platformEvent.Id);
            await unitOfWork.Containers.DeleteAsync(staleContainers.Select(s => s.Id), cancellationToken);
        }

        await unitOfWork.CommitAsync();
        return [.. currentActiveContainers];
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