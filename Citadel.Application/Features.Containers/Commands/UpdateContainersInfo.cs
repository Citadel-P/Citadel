using Application.Features.Containers.Models;
using Application.Services.Abstractions;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Hosting.Common.ErrorTypes;

namespace Application.Features.Containers.Commands;

public sealed record UpdateContainersInfo(ContainersInfoRequest Input) : ICommand<Result>;

internal class UpdateContainersInfoHandler(
    ICacheService cacheService,
    ApplicationDbContext dbContext,
    ILogger<UpdateContainersInfoHandler> logger,
    IContainerHubDispatcher containerHub) : ICommandHandler<UpdateContainersInfo, Result>
{
    public async ValueTask<Result> Handle(UpdateContainersInfo command, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(command.Input.Id, cancellationToken);
        if (platformId == null)
        {
            return Result.Failure(new NotFoundError($"Platform does not exists, daemon id: {command.Input.Id}"));
        }

        var containers = await dbContext.ContainersInfo.Where(s => s.PlatformId == platformId.Value).ToListAsync(cancellationToken);

        var containersToDelete = containers.Where(s => command.Input.ContainersInfo.Select(s => s.Id).Contains(s.ContainerId) == false);
        if (containersToDelete.Any())
        {
            dbContext.ContainersInfo.RemoveRange(containersToDelete);
        }

        foreach (var containerMessage in command.Input.ContainersInfo)
        {
            var existing = containers.SingleOrDefault(s => s.ContainerId == containerMessage.Id);
            if (existing is null)
            {
                logger.LogDebug("Create new record for {ContainerId} ", containerMessage.Id);

                var containerInfo = containerMessage.Map(platformId.Value, command.Input.Created);
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
                    created: command.Input.Created);
                dbContext.ContainerStats.Add(stat);
            }
        }

        await dbContext.SaveChangesAsync(cancellationToken);

        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));

        return Result.Success();
    }
}

internal static class ContainerInfoMapper
{
    public static ContainerInfo Map(this ContainerInfoRequest container, Guid platformId, long? created = null)
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