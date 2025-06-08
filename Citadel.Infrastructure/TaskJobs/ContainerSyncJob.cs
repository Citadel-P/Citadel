using System.Threading.Channels;
using Agent.Server.Containers;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Syncing full containers state when platform status change.
/// </summary>
internal class ContainerSyncJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ILogger<ContainerSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> _platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platformEvent in _platformHealthReader.ReadAllAsync(cancellationToken))
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
        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

        Container[] syncedContainers;

        if (platformEvent.IsOnLine)
        {
            logger.LogInformation("Platform {PlatformId} is online. Syncing containers from {Address}...", platformEvent.Id, platformEvent.Address);
            syncedContainers = await SyncOnlinePlatformContainers(dbContext, platformEvent, cancellationToken);
            UpdateContainerCache(platformEvent.Id, syncedContainers);
        }
        else
        {
            logger.LogInformation("Platform {PlatformId} is offline. Updating container states.", platformEvent.Id);
            syncedContainers = await SyncOfflinePlatformContainers(dbContext, platformEvent.Id, cancellationToken);
            platformContainerCache.EvictPlatform(platformEvent.Id);
        }

        // Notify clients and update the cache
        await containerHub.SendContainersInfo(platformEvent.Id, syncedContainers);

        logger.LogInformation("Successfully synchronized {Count} containers for platform {PlatformId}.", syncedContainers.Length, platformEvent.Id);
    }

    private async Task<Container[]> SyncOnlinePlatformContainers(ApplicationDbContext dbContext, PlatformHealth platformEvent, CancellationToken cancellationToken)
    {
        var containersClient = clientFactory.GetContainerClient(platformEvent.Address);
        var containersReply = await containersClient.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: cancellationToken);

        long fetchTimestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        var freshContainers = containersReply.Containers.ToDictionary(
            c => c.Key,
            c => c.Value.Map(platformEvent.Id, fetchTimestamp)
        );

        var existingContainers = await dbContext.Containers
            .Where(c => c.PlatformId == platformEvent.Id)
            .ToDictionaryAsync(c => c.ContainerId, cancellationToken);

        // Add or Update containers
        foreach (var freshContainer in freshContainers.Values)
        {
            if (existingContainers.TryGetValue(freshContainer.ContainerId, out var existing))
            {
                // Update existing container
                existing.PartialUpdate(
                    name: freshContainer.Name,
                    image: freshContainer.Image,
                    state: freshContainer.State,
                    stack: freshContainer.Stack,
                    created: freshContainer.Created,
                    ports: freshContainer.Ports
                );
            }
            else
            {
                // Add new container
                dbContext.Containers.Add(freshContainer);
            }
        }

        //  Remove stale containers
        var staleContainers = existingContainers.Values
            .Where(c => !freshContainers.ContainsKey(c.ContainerId))
            .ToArray();

        if (staleContainers.Length > 0)
        {
            logger.LogInformation("Removing {Count} stale containers for platform {PlatformId}.", staleContainers.Length, platformEvent.Id);
            dbContext.Containers.RemoveRange(staleContainers);
        }

        await dbContext.SaveChangesAsync(cancellationToken);
        return [.. existingContainers.Values];
    }

    private static async Task<Container[]> SyncOfflinePlatformContainers(ApplicationDbContext dbContext, Guid platformId, CancellationToken cancellationToken)
    {
        var offlineContainers = await dbContext.Containers
            .Where(c => c.PlatformId == platformId)
            .ToArrayAsync(cancellationToken);

        foreach (var container in offlineContainers)
        {
            container.PartialUpdate(state: ContainerStateStatus.Offline);
        }

        await dbContext.SaveChangesAsync(cancellationToken);
        return offlineContainers;
    }

    private void UpdateContainerCache(Guid platformId, Container[] containers)
    {
        var dictionary = containers.ToDictionary(c => c.ContainerId, c => c.Id);
        platformContainerCache.ReplacePlatformContainers(platformId, dictionary);
    }
}