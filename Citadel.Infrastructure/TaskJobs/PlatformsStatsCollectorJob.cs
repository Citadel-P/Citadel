using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Citadel.Agent.Common.V1;
using Citadel.Agent.Platforms.V1;
using Grpc.Core;
using Domain.Entities;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Collects platforms stats from remote agents and pushes into the shared Channel <see cref="PlatformsStatsPersistenceJob"/>.
/// </summary>
internal class PlatformsStatsCollectorJob(
    IGrpcClientFactory clientFactory,
    IOptions<JobConfiguration> options,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ChannelWriter<PlatformStatsBatch> platformStatsWriter,
    ILogger<PlatformsStatsCollectorJob> logger) : BackgroundService
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
            logger.LogWarning("Streaming platform stats for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => StreamPlatformStats(platform, cts.Token), cts.Token);
    }

    public void StopStreamStatsForPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting streaming containers stats for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task StreamPlatformStats(PlatformHealth platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = clientFactory.GetPlatformClient(platform.Address);
                using var stream = client.StreamPlatformStats(new PlatformStatsRequest { FetchIntervalMs = jobConfiguration.SystemInfoInterval * 1000 }, cancellationToken: cancellationToken);
                await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
                {
                    await platformStatsWriter.WriteAsync(
                        new PlatformStatsBatch(
                            PlatformId: platform.Id,
                            NetworksCount: reply.NetworkCount,
                            VolumesCount: reply.VolumeCount,
                            Containers: reply.ContainerCount,
                            ContainersRunning: reply.ContainersRunning,
                            ContainersPaused: reply.ContainersPaused,
                            ContainersStopped: reply.ContainersStopped,
                            Images: reply.ImageCount,
                            MemTotal: reply.MemTotal,
                            CreatePlatformStat(platform.Id, reply.Stat)), 
                        cancellationToken);
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
    private static PlatformStat CreatePlatformStat(Guid platformId, PlatformStatMessage reply)
        => new PlatformStat(
            memoryUsage: reply.MemoryUsage,
            cpuUsage: reply.CpuUsage,
            rxBytes: reply.RxBytes,
            txBytes: reply.TxBytes,
            platformId: platformId,
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds());
}

public sealed record PlatformStatsBatch(
    Guid PlatformId,
    int NetworksCount,
    int VolumesCount,
    long Containers,
    long ContainersRunning,
    long ContainersPaused,
    long ContainersStopped,
    long Images,
    long MemTotal,
    PlatformStat Stat);