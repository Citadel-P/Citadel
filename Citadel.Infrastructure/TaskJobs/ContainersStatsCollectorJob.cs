using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Agent.Server.Containers;
using Citadel.Common;
using Google.Protobuf.Collections;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Collects containers stats from remote agents and pushes into the shared Channel <see cref="ContainersStatsPersistenceJob"/>.
/// </summary>
internal class ContainersStatsCollectorJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    ChannelWriter<ContainersStatBatch> channel,
    ILogger<ContainersStatsCollectorJob> logger,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IOptions<JobConfiguration> options) : BackgroundService
{
    private readonly JobConfiguration jobConfiguration = options.Value;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platform in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            if (platform.IsOnLine)
            {
                StartStreamStatsForPlatform(platform, cancellationToken);
            }
            else
            {
                StopStreamStatsForPlatform(platform.Address);
            }
        }
    }

    public void StartStreamStatsForPlatform(PlatformHealth platform, CancellationToken cancellationToken)
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

    private async Task StreamContainersStats(PlatformHealth platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = clientFactory.GetContainerClient(platform.Address);
                using var stream = client.StreamContainersStats(new ContainersStatsRequest { FetchIntervalMs = jobConfiguration.ContainersInfoInterval * 1000 }, cancellationToken: cancellationToken);
                await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
                {
                    if (reply.Containers.Count > 0)
                    {
                        using var scope = scopeFactory.CreateAsyncScope();
                        using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

                        var ids = await dbContext.ContainersInfo
                                    .AsNoTracking()
                                    .Where(s => s.PlatformId == platform.Id && reply.Containers.Keys.Contains(s.ContainerId))
                                    .ToDictionaryAsync(s => s.ContainerId, s => s.Id, cancellationToken);

                        var stats = ContainerInfoMapper.Map(reply.Containers, ids, DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                        await channel.WriteAsync(new ContainersStatBatch(platform.Id, stats), cancellationToken);
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
}

internal sealed record ContainersStatBatch(Guid PlatformId, ContainerStat[] Stats);

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
