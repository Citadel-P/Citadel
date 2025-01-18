using Application.Services.Abstractions;
using Application.Utils;
using Gcontainers;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal sealed class ContainerService(
    ICacheService cacheService,
    ApplicationDbContext dbContext,
    ILogger<ContainerService> logger,
    IContainerHubDispatcher containerHub) : IContainerService
{
    /// <inheritdoc />
    public async Task OnContainersInfoMessage(ContainersInfoMessage message, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(message.Id, cancellationToken);
        if (platformId == null)
        {
            logger.LogError("Platform does not exists, daemon id: {PlatformId}",  message.Id);
            return;
        }

        var containers = await dbContext.ContainersInfo.Where(s => s.PlatformId == platformId.Value).ToListAsync(cancellationToken);

        var containersToDelete = containers.Where(s => message.ContainersInfo.Select(s => s.Id).Contains(s.ContainerId) == false);
        if (containersToDelete.Any())
        {
            dbContext.ContainersInfo.RemoveRange(containersToDelete);
        }

        foreach (var containerMessage in message.ContainersInfo)
        {
            var existing = containers.SingleOrDefault(s => s.ContainerId == containerMessage.Id);
            if (existing is null)
            {
                logger.LogDebug("Create new record for {ContainerId} ", containerMessage.Id);

                var containerInfo = containerMessage.Map(platformId.Value, message.Created);
                dbContext.ContainersInfo.Add(containerInfo);
                containers.Add(containerInfo);
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
                    created: containerMessage.Created);
                dbContext.ContainerStats.Add(stat);
            }
        }

        await dbContext.SaveChangesAsync(cancellationToken);
        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));
    }

    /// <inheritdoc />
    public async Task OnContainerEvent(ContainerEventMessage message, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(message.Id, cancellationToken);
        if (platformId == null)
        {
            logger.LogError("Platform does not exists, {daemonId}:", message.Id);
        }

        var containerInfo = message.ContainerInfo.Map(platformId.Value);

        if (message.Action == "create")
        {
            dbContext.ContainersInfo.Add(containerInfo);
            await dbContext.SaveChangesAsync(cancellationToken);
        }
        else if (message.Action == "destroy")
        {
            var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == message.ContainerId, cancellationToken);
            dbContext.ContainersInfo.Remove(existing);
            await dbContext.SaveChangesAsync(cancellationToken);
        }
        else
        {
            var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == message.ContainerId, cancellationToken);
            if (existing != null)
            {
                var state = (message.Action) switch
                {
                    "stop" => "exited",
                    "start" => "running",
                    "pause" => "paused",
                    "restart" => "restarting",
                    _ => throw new NotImplementedException()
                };
                existing.UpdateWith(state: state, status: containerInfo.Status);

                await dbContext.SaveChangesAsync(cancellationToken);
            }
        }

        var containers = await dbContext.ContainersInfo.WithLastStat(platformId.Value).ToListAsync(cancellationToken);
        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));
    }

    /// <inheritdoc />
    public async Task OnContainerLogsMessage(ContainerLogMessage message, CancellationToken cancellationToken)
    {
        await containerHub.SendContainerLogs(message);
    }
}


internal static class ContainerInfoMapper
{
    public static ContainerInfo Map(this ContainerInfoMessage container, Guid platformId, long? created = null)
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
                    created: created ?? DateTimeOffset.UtcNow.ToUnixTimeSeconds());

        containerInfo.Stats.Add(stat);

        return containerInfo;
    }
}
