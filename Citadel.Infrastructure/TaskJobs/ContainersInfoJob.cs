using System;
using System.Collections.Concurrent;
using Agent.Server.Containers;
using Citadel.Common;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class ContainersInfoJob(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<ContainersInfoJob> logger,
    IContainerHubDispatcher containerHub) : IJob
{
    public static readonly JobKey JobKey = new(nameof(ContainersInfoJob), "Containers");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var platforms = await dbContext.Platforms.AsNoTracking()
            .Select(s => new { s.Id, s.DaemonId, s.Address, s.Status})
            .ToListAsync(context.CancellationToken);

        if (platforms.Count == 0) return;

        var platformsData = await GetPlatformsData(platforms.Where(s => s.Status == PlatformStatus.Online).Select(s => s.Address), context.CancellationToken);
        foreach (var platform in platforms)
        {
            var platformData = platformsData.SingleOrDefault(s => s.PlatformAddress == platform.Address);
            var containers = await dbContext.ContainersInfo.Where(s => s.PlatformId == platform.Id).ToListAsync(context.CancellationToken);

            if (platformData != null && platformData.Status == PlatformStatus.Online)
            {
                var containersToDelete = containers.Where(s => platformData.Containers.Select(s => s.Id).Contains(s.ContainerId) == false);
                if (containersToDelete.Any())
                {
                    dbContext.ContainersInfo.RemoveRange(containersToDelete);
                }

                foreach (var containerMessage in platformData.Containers)
                {
                    var existing = containers.SingleOrDefault(s => s.ContainerId == containerMessage.Id);
                    if (existing is null)
                    {
                        logger.LogDebug("Create new record for {ContainerId} ", containerMessage.Id);

                        var containerInfo = containerMessage.Map(platform.Id);
                        dbContext.ContainersInfo.Add(containerInfo);
                        containers.Add(containerInfo);
                    }
                    else
                    {
                        logger.LogDebug("Update record for {ContainerId} ", containerMessage.Id);

                        existing.PartialUpdate(
                            name: containerMessage.Name,
                            image: containerMessage.Image,
                            created: containerMessage.Created,
                            state: containerMessage.State,
                            status: containerMessage.Status,
                            labels: containerMessage.Labels.ToDictionary(),
                            ports: containerMessage.Ports.Map());

                        var stat = ContainerStat.Create(
                            containerInfoId: existing.Id,
                            memoryUsage: containerMessage.ContainerStatMessage?.MemoryUsage,
                            memoryLimit: containerMessage.ContainerStatMessage?.MemoryLimit,
                            cpuUsage: containerMessage.ContainerStatMessage?.CpuUsage,
                            rxBytes: containerMessage.ContainerStatMessage?.RxBytes,
                            txBytes: containerMessage.ContainerStatMessage?.TxBytes,
                            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                        dbContext.ContainerStats.Add(stat);
                    }
                }
            }
            else
            {
                foreach (var container in containers) 
                {
                    container.PartialUpdate(state: "offline");
                }
            }
    
            await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));
        }
        await dbContext.SaveChangesAsync(context.CancellationToken);
    }

    private async Task<IEnumerable<PlatformData>> GetPlatformsData(IEnumerable<string> addresses, CancellationToken cancellationToken)
    {
        var containersData = new ConcurrentBag<PlatformData>();
        await Parallel.ForEachAsync(addresses, cancellationToken, async (address, cancellationToken) =>
        {
            var client = clientFactory.GetContainerClient(address);
            try
            {
                var containers = await client.ListContainersAsync(new ContainersListMessage() { All = true }, cancellationToken: cancellationToken);
                containersData.Add(new PlatformData(address, PlatformStatus.Online, containers.Containers));
            }
            catch (RpcException ex)
            {
                logger.LogWarning(ex, "Error while getting containers info from platform {PlatformAddress}", address);
                containersData.Add(new PlatformData(address, PlatformStatus.Offline));
            }
        });
        return containersData;
    }

    private record PlatformData(string PlatformAddress, PlatformStatus Status, IEnumerable<ContainerMessage> Containers = null);
}

internal static class ContainerInfoMapper
{
    public static ContainerInfo Map(this ContainerMessage container, Guid platformId)
    {
        var containerInfo = ContainerInfo.Create(
                    platformId: platformId,
                    containerId: container.Id,
                    name: container.Name,
                    image: container.Image,
                    created: container.Created,
                    state: container.State,
                    status: container.Status,
                    ports: container.Ports?.Map(),
                    labels: container.Labels.ToDictionary());

        var stat = ContainerStat.Create(
                    containerInfoId: containerInfo.Id,
                    memoryUsage: container.ContainerStatMessage?.MemoryUsage,
                    memoryLimit: container.ContainerStatMessage?.MemoryLimit,
                    cpuUsage: container.ContainerStatMessage?.CpuUsage,
                    rxBytes: container.ContainerStatMessage?.RxBytes,
                    txBytes: container.ContainerStatMessage?.TxBytes,
                    created: DateTimeOffset.UtcNow.ToUnixTimeSeconds());

        containerInfo.Stats.Add(stat);
        return containerInfo;
    }

    public static ICollection<ContainerPort> Map(this IEnumerable<PortMessage> ports) 
        => [.. ports.Select(Map)];

    public static ContainerPort Map(this PortMessage port)
        => new (port.IP, port.PrivatePort, port.PublicPort);
}