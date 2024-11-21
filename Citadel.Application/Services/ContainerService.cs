using Application.Services.Abstractions;
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
    public async Task OnContainersInfoMessage(ContainersInfoMessage message, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(message.ID, cancellationToken);

        if (platformId != null)
        {
            var containers = await dbContext.ContainersInfo.Where(s => s.PlatformId == platformId.Value).ToListAsync(cancellationToken);

            var containersToDelete = containers.Where(s => message.ContainerListMessages.Select(s => s.Id).Contains(s.ContainerId) == false);
            if (containersToDelete.Any())
            {
                dbContext.ContainersInfo.RemoveRange(containersToDelete);
            }

            foreach (var containerMessage in message.ContainerListMessages)
            {
                var existing = containers.SingleOrDefault(s => s.ContainerId == containerMessage.Id);
                if (existing is null) 
                {
                    logger.LogDebug("Create new record for {ContainerId} ", containerMessage.Id);
                    var containerInfo = ContainerInfo.Create(
                    platformId: platformId.Value,
                    containerId: containerMessage.Id,
                    name: containerMessage.Names.First(),
                    image: containerMessage.Image,
                    created: containerMessage.Created,
                    state: containerMessage.State,
                    status: containerMessage.Status,
                    ports: containerMessage.Ports.Map(),
                    labels: containerMessage.Labels);
                    dbContext.ContainersInfo.Add(containerInfo);

                    var stat = ContainerStat.Create(
                        containerInfoId: containerInfo.Id,
                        memoryUsage: containerMessage.ContainerStat?.MemoryUsage,
                        memoryLimit: containerMessage.ContainerStat?.MemoryLimit,
                        cpuUsage: containerMessage.ContainerStat?.CpuUsage,
                        rxBytes: containerMessage.ContainerStat?.RxBytes,
                        txBytes: containerMessage.ContainerStat?.TxBytes,
                        createdAtUtc: containerMessage.ContainerStat?.CreatedAtUtc);
                    dbContext.ContainerStats.Add(stat);
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
                        createdAtUtc: containerMessage.ContainerStat?.CreatedAtUtc);
                    dbContext.ContainerStats.Add(stat);
                }
            }

            await dbContext.SaveChangesAsync(cancellationToken);

            await containerHub.SendContainersInfo(platformId.Value, containers.OrderByDescending(s => s.Created));
        }
    }

    /// <inheritdoc />
    public async Task OnContainerLogsMessage(ContainerLogMessage message, CancellationToken cancellationToken) 
        => await containerHub.SendContainerLogs(message);
    
}