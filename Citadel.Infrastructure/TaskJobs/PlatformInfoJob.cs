using System.Collections.Concurrent;
using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using Agent.Server.GPlatform;
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

public interface IPlatformInfoJob : IHostedService
{
    void StartStreamStatsForPlatform(PlatformData platform, CancellationToken cancellationToken);
    void StopStreamStatsForPlatform(string address);
}

internal class PlatformInfoJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    ILogger<PlatformInfoJob> logger) : BackgroundService, IPlatformInfoJob
{
    private readonly JobConfiguration jobConfiguration = options.Value;
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

        var platforms = await dbContext.Platforms.AsNoTracking()
                 .Select(s => new PlatformData(s.Id, s.Address, s.Status))
                 .ToArrayAsync(cancellationToken);

        foreach (var platform in platforms)
        {
            StartStreamStatsForPlatform(platform, cancellationToken);
        }
    }

    public void StartStreamStatsForPlatform(PlatformData platform, CancellationToken cancellationToken)
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

    private async Task StreamPlatformStats(PlatformData platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var client = clientFactory.GetPlatformClient(platform.Address);
                using var stream = client.StreamPlatformStats(new PlatformStatsRequest { FetchIntervalMs = jobConfiguration.SystemInfoInterval * 1000 }, cancellationToken: cancellationToken);
                await foreach (var reply in stream.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
                {
                    using var scope = scopeFactory.CreateAsyncScope();
                    var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
                    var platformHub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();

                    var existing = await dbContext.Platforms.FirstOrDefaultAsync(p => p.Address == platform.Address, cancellationToken);
                    if (existing == null)
                    {
                        logger.LogWarning("Platform with address {Address} not found in the database.", platform.Address);
                        return;
                    }

                    existing.PartialUpdate(
                        platformStatus: PlatformStatus.Online,
                        networksCount: reply.NetworksCount,
                        volumesCount: reply.VolumesCount,
                        containers: reply.Containers,
                        containersRunning: reply.ContainersRunning,
                        containersPaused: reply.ContainersPaused,
                        containersStopped: reply.ContainersStopped,
                        images: reply.Images,
                        memTotal: reply.MemTotal);

                    dbContext.PlatformStats.Add(CreatePlatformStat(existing.Id, reply.Stat));

                    await dbContext.SaveChangesAsync(cancellationToken);
                    await platformHub.PushPlatformUpdate(existing);
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
        => PlatformStat.Create(
            memoryUsage: reply.MemoryUsage,
            cpuUsage: reply.CpuUsage,
            rxBytes: reply.RxBytes,
            txBytes: reply.TxBytes,
            platformId: platformId,
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds());
}