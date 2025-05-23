using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using Citadel.Common;
using Google.Protobuf.Collections;
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

public interface IContainersStatsJob
{
    void StartStreamStatsForPlatform(PlatformData platform, CancellationToken cancellationToken);
    void StopStreamStatsForPlatform(string address);
}

internal class ContainersStatsJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    ILogger<ContainersStatsJob> logger,
    IOptions<JobConfiguration> options) : BackgroundService, IContainersStatsJob
{
    private readonly JobConfiguration jobConfiguration = options.Value;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platforms = await dbContext.Platforms.AsNoTracking()
                 .Select(s => new PlatformData(s.Id, s.Address, s.Status))
                 .ToListAsync(cancellationToken);

        foreach (var platform in platforms)
        {
            StartStreamStatsForPlatform(platform, cancellationToken);
        }
    }

    public void StartStreamStatsForPlatform(PlatformData platform, CancellationToken cancellationToken)
    {
        if (runningStreams.ContainsKey(platform.Address))
        {
            logger.LogWarning("Streaming containers stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => StreamContainersStats(platform, cts.Token), cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamContainersStats(PlatformData platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = clientFactory.GetContainerClient(platform.Address);
                using var stream = client.StreamContainersStats(new ContainersStatsRequest { FetchIntervalMs = jobConfiguration.ContainersInfoInterval * 1000 }, cancellationToken: cancellationToken);
                await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
                {
                    if (reply .Containers.Count > 0)
                    {
                        using var scope = scopeFactory.CreateAsyncScope();
                        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                        var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

                        var ids = await dbContext.ContainersInfo
                                    .AsNoTracking()
                                    .Where(s => s.PlatformId == platform.Id && reply.Containers.Keys.Contains(s.ContainerId))
                                    .ToDictionaryAsync(s => s.ContainerId, s => s.Id, cancellationToken);

                        var stats = ContainerInfoMapper.Map(reply.Containers, ids, DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                        dbContext.ContainerStats.AddRange(stats);

                        await dbContext.SaveChangesAsync(cancellationToken);
                        await containerHub.SendContainersStats(stats, platform.Id);
                    }
                }
            }
            catch (RpcException ex) when (ex.StatusCode == StatusCode.Cancelled)
            {
                logger.LogInformation("Stream for {Address} was canceled.", platform.Address);
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while streaming containers stats for {Address}, retrying in 10s...", platform.Address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
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
}

public readonly struct PlatformData(Guid Id, string Address, PlatformStatus Status)
{
    public Guid Id { get; } = Id;
    public string Address { get; } = Address;
    public PlatformStatus Status { get; } = Status;
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
    public static ContainerStat[] Map(this MapField<string, ContainerStatMessage> stats, Dictionary<string, Guid> ids, long timestamp)
    {
        var temp = new ContainerStat[stats.Count];
        var index = 0;

        foreach (var kvp in stats)
        {
            if (ids.TryGetValue(kvp.Key, out var containerId))
            {
                temp[index++] = ContainerStat.Create(
                    containerInfoId: containerId,
                    memoryUsage: kvp.Value.MemoryUsage,
                    memoryLimit: kvp.Value.MemoryLimit,
                    cpuUsage: kvp.Value.CpuUsage,
                    rxBytes: kvp.Value.RxBytes,
                    txBytes: kvp.Value.TxBytes,
                    created: timestamp);
            }
        }

        // Trim excess if some keys weren't matched
        return index == temp.Length
            ? temp
            : temp[..index];
    }

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
