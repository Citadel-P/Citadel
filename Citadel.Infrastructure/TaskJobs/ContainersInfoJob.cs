using System.Buffers;
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
            .Select(p => new PlatformStub(p.Id, p.DaemonId, p.Address, p.Status))
            .ToListAsync(context.CancellationToken);

        if (platforms.Count == 0) return;

        var online = new List<PlatformStub>(platforms.Count);
        foreach (var p in platforms)
        {
            if (p.Status == PlatformStatus.Online)
                online.Add(p);
        }

        var addresses = online.Select(p => p.Address);
        var platformDataMap = (await GetPlatformsData(addresses, context.CancellationToken))
            .ToDictionary(p => p.PlatformAddress);

        foreach (var platform in platforms)
        {
            var containers = await dbContext.ContainersInfo
                .Where(c => c.PlatformId == platform.Id)
                .ToListAsync(context.CancellationToken);

            if (platformDataMap.TryGetValue(platform.Address, out var platformData) &&
                platformData.Status == PlatformStatus.Online)
            {
                var activeIds = new HashSet<string>(StringComparer.Ordinal);
                foreach (var c in platformData.Containers)
                    activeIds.Add(c.Id);

                var pool = ArrayPool<ContainerInfo>.Shared;
                var rented = pool.Rent(containers.Count);
                int deleteCount = 0;

                foreach (var c in containers)
                {
                    if (!activeIds.Contains(c.ContainerId))
                        rented[deleteCount++] = c;
                }

                if (deleteCount > 0)
                    dbContext.ContainersInfo.RemoveRange(rented.AsSpan(0, deleteCount).ToArray());

                pool.Return(rented, clearArray: true);

                var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

                foreach (var msg in platformData.Containers)
                {
                    var existing = containers.SingleOrDefault(c => c.ContainerId == msg.Id);
                    if (existing == null)
                    {
                        logger.LogDebug("Create new record for {ContainerId}", msg.Id);
                        var newInfo = msg.Map(platform.Id);
                        dbContext.ContainersInfo.Add(newInfo);
                        containers.Add(newInfo);
                    }
                    else
                    {
                        logger.LogDebug("Update record for {ContainerId}", msg.Id);
                        existing.PartialUpdate(
                            name: msg.Name,
                            image: msg.Image,
                            created: msg.Created,
                            state: msg.State,
                            status: msg.Status,
                            labels: msg.Labels.ToDictionary(),
                            ports: msg.Ports.Map());

                        dbContext.ContainerStats.Add(ContainerStat.Create(
                            containerInfoId: existing.Id,
                            memoryUsage: msg.ContainerStatMessage?.MemoryUsage,
                            memoryLimit: msg.ContainerStatMessage?.MemoryLimit,
                            cpuUsage: msg.ContainerStatMessage?.CpuUsage,
                            rxBytes: msg.ContainerStatMessage?.RxBytes,
                            txBytes: msg.ContainerStatMessage?.TxBytes,
                            created: timestamp));
                    }
                }
            }
            else
            {
                foreach (var c in containers)
                    c.PartialUpdate(state: "offline");
            }

            containers.Sort((a, b) => b.Created.CompareTo(a.Created));
            // Notify client(s)
            await containerHub.SendContainersInfo(containers);
        }

        await dbContext.SaveChangesAsync(context.CancellationToken);
    }

    private async Task<IEnumerable<PlatformData>> GetPlatformsData(IEnumerable<string> addresses, CancellationToken cancellationToken)
    {
        var containersData = new ConcurrentBag<PlatformData>();
        await Parallel.ForEachAsync(addresses, cancellationToken, async (address, ct) =>
        {
            var client = clientFactory.GetContainerClient(address);
            try
            {
                var containers = await client.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: ct);
                containersData.Add(new PlatformData(address, PlatformStatus.Online, containers.Containers));
            }
            catch (RpcException ex)
            {
                logger.LogWarning(ex, "Error while getting containers info from {PlatformAddress}", address);
                containersData.Add(new PlatformData(address, PlatformStatus.Offline));
            }
        });
        return containersData;
    }

    private readonly record struct PlatformStub(Guid Id, string DaemonId, string Address, PlatformStatus Status);
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
        => new(port.IP, port.PrivatePort, port.PublicPort);
}
