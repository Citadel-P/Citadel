using Application.Services.Abstractions;
using Application.Utils;
using Contracts.Broker.Models;
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
    public async Task OnContainersInfoMessage(ContainerListMessage message, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(message.ID, cancellationToken);
        if (platformId == null) return;
        
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
                    name: containerMessage.Names.First(),
                    image: containerMessage.Image,
                    created: containerMessage.Created,
                    state: containerMessage.State,
                    status: containerMessage.Status,
                    labels: containerMessage.Labels,
                    ports: containerMessage.Ports.Map());

                var stat = ContainerStat.Create(
                    containerInfoId: existing.Id,
                    memoryUsage: containerMessage.ContainerStat?.MemoryUsage,
                    memoryLimit: containerMessage.ContainerStat?.MemoryLimit,
                    cpuUsage: containerMessage.ContainerStat?.CpuUsage,
                    rxBytes: containerMessage.ContainerStat?.RxBytes,
                    txBytes: containerMessage.ContainerStat?.TxBytes,
                    created: message.Created);
                dbContext.ContainerStats.Add(stat);
            }
        }

        await dbContext.SaveChangesAsync(cancellationToken);

        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));   
    }

    /// <inheritdoc />
    public async Task OnContainerLogsMessage(ContainerLogMessage message, CancellationToken cancellationToken) 
        => await containerHub.SendContainerLogs(message);

    /// <inheritdoc />
    public async Task OnContainerEventMessage(ContainerEventMessage message, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(message.ID, cancellationToken);
        if (platformId == null) return;

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
                    _ => throw new NotImplementedException()
                };

                existing.UpdateWith(state: state, status: containerInfo.Status);

                await dbContext.SaveChangesAsync(cancellationToken);
            }
        }

        var containers = await dbContext.ContainersInfo.WithLastStat(platformId.Value).ToListAsync(cancellationToken);
        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));
    }
}

internal static class ContainerInfoMapper 
{
    public static ContainerInfo Map(this ContainerInfoMessage container, Guid platformId, long? created = null)
    {
        var containerInfo = ContainerInfo.Create(
                    platformId: platformId,
                    containerId: container.Id,
                    name: container.Names?.First(),
                    image: container.Image,
                    created: container.Created,
                    state: container.State,
                    status: container.Status,
                    ports: container.Ports?.Map(),
                    labels: container.Labels);

        var stat = ContainerStat.Create(
                    containerInfoId: containerInfo.Id,
                    memoryUsage: container.ContainerStat?.MemoryUsage,
                    memoryLimit: container.ContainerStat?.MemoryLimit,
                    cpuUsage: container.ContainerStat?.CpuUsage,
                    rxBytes: container.ContainerStat?.RxBytes,
                    txBytes: container.ContainerStat?.TxBytes,
                    created: created ?? DateTimeOffset.UtcNow.ToUnixTimeSeconds());

        containerInfo.Stats.Add(stat);

        return containerInfo;
    }
}