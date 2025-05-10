using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using Citadel.Common;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

internal class ContainersInfoJob(
    IGrpcClientFactory clientFactory,
    IServiceProvider serviceProvider,
    ILogger<ContainersInfoJob> logger,
    IOptions<JobConfiguration> options) : BackgroundService
{
    private readonly JobConfiguration jobConfiguration = options.Value;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                using var scope = serviceProvider.CreateScope();
                var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();
                await RunJob(dbContext, containerHub, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Unhandled exception in ContainersInfoJob");
            }

            await Task.Delay(TimeSpan.FromSeconds(jobConfiguration.ContainersInfoInterval), cancellationToken);
        }
    }

    public async Task RunJob(ApplicationDbContext dbContext, IContainerHubDispatcher containerHub, CancellationToken cancellationToken)
    {
        var platforms = await dbContext.Platforms
            .FromSqlRaw("SELECT Id, Address, Status FROM Platforms").AsNoTracking()
            .Select(s => new PlatformData(s.Id, s.Address, s.Status))
            .ToArrayAsync(cancellationToken);

        if (platforms.Length == 0) return;

        foreach (var platform in platforms)
        {
            await ProcessPlatform(dbContext, containerHub, platform, cancellationToken);
        }
    }

    private async Task ProcessPlatform(ApplicationDbContext dbContext, IContainerHubDispatcher containerHub, PlatformData platform, CancellationToken ct)
    {
        try
        {
            var containers = await dbContext.ContainersInfo
                .Where(c => c.PlatformId == platform.Id)
                .OrderByDescending(c => c.Created)
                .ToDictionaryAsync(c => c.ContainerId, ct);

            if (platform.Status == PlatformStatus.Online)
            {
                await SyncContainers(dbContext, platform, containers, ct);
            }
            else
            {
                // Mark containers as offline
                foreach (var container in containers.Values)
                {
                    container.PartialUpdate(state: ContainerStateStatus.Offline);
                }
            }

            await dbContext.SaveChangesAsync(ct);

            if(containers.Count != 0)
                await containerHub.SendContainersInfo(containers.Values);
        }
        catch (RpcException ex)
        {
            logger.LogWarning(ex, "Error while syncing containers from platform {PlatformId}", platform.Id);
        }
    }

    private async Task SyncContainers(ApplicationDbContext dbContext, PlatformData platform, Dictionary<string, ContainerInfo> containers, CancellationToken ct)
    {
        var client = clientFactory.GetContainerClient(platform.Address);
        var response = await client.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: ct);
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var containersMsg = response.Containers;

        // Remove stale containers
        var staleContainers = containers
            .Where(kvp => !containersMsg.ContainsKey(kvp.Key))
            .Select(kvp => kvp.Value)
            .ToList();

        if (staleContainers.Count != 0)
        {
            dbContext.ContainersInfo.RemoveRange(staleContainers);
        }

        // Add or update containers
        var containerStats = containersMsg.Count > 0
            ? new List<ContainerStat>(containersMsg.Count(s => s.Value.State == ContainerStateType.Running))
            : null;

        foreach (var msg in containersMsg)
        {
            if (!containers.TryGetValue(msg.Key, out var existing))
            {
                dbContext.ContainersInfo.Add(msg.Value.Map(platform.Id, timestamp));
            }
            else
            {
                UpdateExistingContainer(existing, msg.Value);

                if (msg.Value.ContainerStatMessage is not null)
                {
                    containerStats?.Add(CreateContainerStat(
                        existing.Id,
                        msg.Value.ContainerStatMessage,
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

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void UpdateExistingContainer(ContainerInfo existing, ContainerMessage msg)
    {
        existing.PartialUpdate(
            name: msg.Name,
            image: msg.Image,
            created: msg.Created,
            state: msg.State.Map(),
            status: msg.Status,
            stack: msg.Stack,
            ports: msg.Ports.Map());
    }

    private readonly struct PlatformData(Guid id, string address, PlatformStatus status)
    {
        public Guid Id { get; } = id;
        public string Address { get; } = address;
        public PlatformStatus Status { get; } = status;
    }
}

public static class ContainerInfoMapper
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
                    state: container.State.Map(),
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
    public static ContainerStateStatus Map(this ContainerStateType state)
        => state switch
        {
            ContainerStateType.Unknown => ContainerStateStatus.Unknown,
            ContainerStateType.Running => ContainerStateStatus.Running,
            ContainerStateType.Paused => ContainerStateStatus.Paused,
            ContainerStateType.Restarting => ContainerStateStatus.Restarting,
            ContainerStateType.Dead => ContainerStateStatus.Dead,
            ContainerStateType.Exited => ContainerStateStatus.Exited,
            ContainerStateType.Removing => ContainerStateStatus.Removing,
            _ => ContainerStateStatus.Unknown,
        };

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ICollection<ContainerPort> Map(this IEnumerable<PortMessage> ports)
        => [.. ports.Select(Map)];

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerPort Map(this PortMessage port)
        => new(port.IP, port.PrivatePort, port.PublicPort);
}
