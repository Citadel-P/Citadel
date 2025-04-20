using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using Citadel.Common;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class ContainersInfoJob(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<ContainersInfoJob> logger,
    IServiceProvider serviceProvider,
    IContainerHubDispatcher containerHub) : IJob
{
    public static readonly JobKey JobKey = new(nameof(ContainersInfoJob), "Containers");
    private static readonly ParallelOptions _parallelOptions = new()
    {
        MaxDegreeOfParallelism = Environment.ProcessorCount
    };

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var platforms = await dbContext.Platforms.AsNoTracking()
            .Select(s => new PlatformData(s.Id, s.Address, s.Status))
            .ToArrayAsync(context.CancellationToken);

        if (platforms.Length == 0) return;

        // Reuse parallel options to avoid allocation but update cancellation token
        _parallelOptions.CancellationToken = context.CancellationToken;

        await Parallel.ForEachAsync(platforms, _parallelOptions, ProcessPlatform);
    }

    private async ValueTask ProcessPlatform(PlatformData platform, CancellationToken ct)
    {
        using var scope = serviceProvider.CreateScope();
        var scopedDb = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        try
        {
            var containers = await scopedDb.ContainersInfo
                .Where(c => c.PlatformId == platform.Id)
                .OrderByDescending(c => c.Created)
                .ToArrayAsync(ct);

            if (platform.Status == PlatformStatus.Online)
            {
                await SyncContainers(platform, containers, scopedDb, ct);
            }
            else
            {
                // Mark containers as offline
                for (int i = 0; i < containers.Length; i++)
                    containers[i].PartialUpdate(state: "offline");
            }

            await scopedDb.SaveChangesAsync(ct);
            await containerHub.SendContainersInfo(containers);
        }
        catch (RpcException ex)
        {
            logger.LogWarning(ex, "Error while syncing containers from platform {PlatformId}", platform.Id);
        }
    }

    private async Task SyncContainers(PlatformData platform, ContainerInfo[] containers, ApplicationDbContext dbContext, CancellationToken ct)
    {
        // Preallocate dictionary with capacity to avoid resizing
        var existingById = new Dictionary<string, ContainerInfo>(containers.Length, StringComparer.Ordinal);
        for (int i = 0; i < containers.Length; i++)
        {
            existingById[containers[i].ContainerId] = containers[i];
        }

        var client = clientFactory.GetContainerClient(platform.Address);
        var response = await client.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: ct);

        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        // Use HashSet with initial capacity to avoid resizing
        var activeIds = new HashSet<string>(response.Containers.Count, StringComparer.Ordinal);
        foreach (var container in response.Containers)
        {
            activeIds.Add(container.Id);
        }

        RemoveStaleContainers(dbContext, containers, activeIds);

        // Preallocate collection with expected capacity
        var containerStats = response.Containers.Count > 0
            ? new List<ContainerStat>(response.Containers.Count)
            : null;

        foreach (var msg in response.Containers)
        {
            if (!existingById.TryGetValue(msg.Id, out var existing))
            {
                dbContext.ContainersInfo.Add(msg.Map(platform.Id, timestamp));
            }
            else
            {
                UpdateExistingContainer(existing, msg);

                if (msg.ContainerStatMessage is not null)
                {
                    containerStats?.Add(CreateContainerStat(
                        existing.Id,
                        msg.ContainerStatMessage,
                        timestamp));
                }
            }
        }

        if (containerStats?.Count > 0)
        {
            dbContext.ContainerStats.AddRange(containerStats);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static ContainerStat CreateContainerStat(Guid containerId, ContainerStatMessage statMsg, long timestamp)
    {
        return ContainerStat.Create(
            containerInfoId: containerId,
            memoryUsage: statMsg.MemoryUsage,
            memoryLimit: statMsg.MemoryLimit,
            cpuUsage: statMsg.CpuUsage,
            rxBytes: statMsg.RxBytes,
            txBytes: statMsg.TxBytes,
            created: timestamp);
    }

    private static void UpdateExistingContainer(ContainerInfo existing, ContainerMessage msg)
    {
        existing.PartialUpdate(
            name: msg.Name,
            image: msg.Image,
            created: msg.Created,
            state: msg.State,
            status: msg.Status,
            stack: msg.Stack,
            ports: msg.Ports.Map());
    }

    private static void RemoveStaleContainers(ApplicationDbContext db, ContainerInfo[] containers, HashSet<string> activeIds)
    {
        List<ContainerInfo> toRemove = null;
        for (int i = 0; i < containers.Length; i++)
        {
            var container = containers[i];
            if (!activeIds.Contains(container.ContainerId))
            {
                toRemove ??= [];
                toRemove.Add(container);
            }
        }

        if (toRemove?.Count > 0)
        {
            db.ContainersInfo.RemoveRange(toRemove);
        }
    }

    private readonly struct PlatformData(Guid id, string address, PlatformStatus status)
    {
        public Guid Id { get; } = id;
        public string Address { get; } = address;
        public PlatformStatus Status { get; } = status;
    }
}

internal static class ContainerInfoMapper
{
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerInfo Map(this ContainerMessage container, Guid platformId, long timestamp)
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
                    stack: container.Stack);

        if (container.ContainerStatMessage is not null)
        {
            containerInfo.Stats.Add(container.ContainerStatMessage.Map(containerInfo.Id, timestamp));
        }

        return containerInfo;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerStat Map(this ContainerStatMessage stat, Guid containerInfoId, long timestamp)
        => ContainerStat.Create(
            containerInfoId: containerInfoId,
            memoryUsage: stat.MemoryUsage,
            memoryLimit: stat.MemoryLimit,
            cpuUsage: stat.CpuUsage,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes,
            created: timestamp);

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ICollection<ContainerPort> Map(this IEnumerable<PortMessage> ports)
        => [.. ports.Select(Map)];

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerPort Map(this PortMessage port)
        => new(port.IP, port.PrivatePort, port.PublicPort);
}
