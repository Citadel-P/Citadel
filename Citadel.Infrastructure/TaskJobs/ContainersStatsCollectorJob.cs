using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Citadel.Agent.Common.V1;
using Citadel.Agent.Containers.V1;
using Google.Protobuf.Collections;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Collects containers stats from remote agents and pushes into the shared Channel <see cref="ContainersStatsPersistenceJob"/>.
/// </summary>
internal class ContainersStatsCollectorJob(
    IGrpcClientFactory clientFactory,
    IOptions<JobConfiguration> options, 
    ChannelWriter<ContainersStatBatch> channel,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ILogger<ContainersStatsCollectorJob> logger) : BackgroundService
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
                using var stream = client.StreamContainerStats(new ContainerStatsRequest { FetchIntervalMs = jobConfiguration.ContainersInfoInterval * 1000 }, cancellationToken: cancellationToken);
                await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
                {
                    if (reply.Containers.Count > 0)
                    {
                        
                        if (platformContainerCache.TryGetContainers(platform.Id, out var ids))
                        {
                            var stats = ContainerMapper.Map(reply.Containers, ids, DateTimeOffset.UtcNow.ToUnixTimeSeconds());
                            await channel.WriteAsync(new ContainersStatBatch(platform.Id, stats), cancellationToken);
                        }
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

public static class ContainerMapper
{
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static Container Map(this ContainerMessage message, Guid platformId, long timestamp)
    {
        var container = new Container(
                    platformId: platformId,
                    containerId: message.Id,
                    name: message.Name,
                    image: message.Image,
                    created: message.Created,
                    state: message.State.Map(),
                    ports: message.Ports?.Map(),
                    stack: message.Stack);

        if (message.ContainerStatMessage is not null)
        {
            container.AppendStat(message.ContainerStatMessage.Map(container.Id, timestamp));
        }

        return container;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerStat Map(this ContainerStatMessage stat, Guid containerId, long timestamp)
        => new (
            containerId: containerId,
            memoryUsage: stat.MemoryUsage,
            memoryLimit: stat.MemoryLimit,
            cpuUsage: stat.CpuUsage,
            rxBytes: stat.RxBytes,
            txBytes: stat.TxBytes,
            created: timestamp);

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static ContainerStat[] Map(this MapField<string, ContainerStatMessage> stats, IReadOnlyDictionary<string, Guid> ids, long timestamp)
    {
        var temp = new ContainerStat[stats.Count];
        var index = 0;

        foreach (var kvp in stats)
        {
            if (ids.TryGetValue(kvp.Key, out var containerId))
            {
                temp[index++] = new (
                    containerId: containerId,
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
