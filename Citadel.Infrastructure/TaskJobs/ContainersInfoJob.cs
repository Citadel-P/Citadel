using Agent.Server.Containers;
using Citadel.Common;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class ContainersInfoJob(
    ICacheService cacheService,
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<ContainersInfoJob> logger,
    IContainerHubDispatcher containerHub) : IJob
{
    public static readonly JobKey JobKey = new(nameof(ContainersInfoJob), "Containers");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var addresses = await cacheService.GetClientsAddresses(context.CancellationToken);
        if (!addresses.Any()) return;

        var replies = new List<ContainersListReply>();
        await Parallel.ForEachAsync(addresses, async (address, token) =>
        {
            var client = clientFactory.GetContainerClient(address);
            try
            {
                var reply = await client.ListContainersAsync(new ContainersListMessage() { All = true }, cancellationToken: context.CancellationToken);
                replies.AddRange(reply);
            }
            catch (RpcException ex)
            {
                logger.LogError(ex, "Error while getting system info from platform {PlatformAddress}", address);
            }
        });

        foreach (var reply in replies)
        {
            Guid? platformId = await cacheService.GetPlatformId(reply.Id, context.CancellationToken);
            if (platformId == null)
            {
                logger.LogError("Platform does not exists, daemon id: {PlatformId}", reply.Id);
                continue;
            }

            var containersList = await dbContext.ContainersInfo.Where(s => s.PlatformId == platformId.Value).ToListAsync(context.CancellationToken);
            var containersToDelete = containersList.Where(s => reply.Containers.Select(s => s.Id).Contains(s.ContainerId) == false);
            if (containersToDelete.Any())
            {
                dbContext.ContainersInfo.RemoveRange(containersToDelete);
            }

            foreach (var containerMessage in reply.Containers)
            {
                var existing = containersList.SingleOrDefault(s => s.ContainerId == containerMessage.Id);
                if (existing is null)
                {
                    logger.LogDebug("Create new record for {ContainerId} ", containerMessage.Id);

                    var containerInfo = containerMessage.Map(platformId.Value);
                    dbContext.ContainersInfo.Add(containerInfo);
                    containersList.Add(containerInfo);
                }
                else
                {
                    logger.LogDebug("Update record for {ContainerId} ", containerMessage.Id);

                    existing.UpdateWith(
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
            await containerHub.SendContainersInfo(containersList.OrderByDescending(s => s.Created));
        }
        await dbContext.SaveChangesAsync(context.CancellationToken);
    }
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