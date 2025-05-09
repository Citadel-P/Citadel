using System.Runtime.CompilerServices;
using Citadel.Common;
using Google.Protobuf.WellKnownTypes;
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

internal class PlatformInfoJob(
    IGrpcClientFactory clientFactory,
    IServiceProvider serviceProvider,
    IOptions<JobConfiguration> options,
    ILogger<PlatformInfoJob> logger) : BackgroundService
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
                var platformHub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();
                await RunJob(dbContext, platformHub, cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Unhandled exception in PlatformInfoJob");
            }

            await Task.Delay(TimeSpan.FromSeconds(jobConfiguration.SystemInfoInterval), cancellationToken);
        }
    }


    public async Task RunJob(ApplicationDbContext dbContext, IPlatformHubDispatcher platformHub, CancellationToken cancellationToken)
    {
        var platforms = await dbContext.Platforms
            .OrderByDescending(s => s.Name)
            .ToArrayAsync(cancellationToken);

        if (platforms.Length == 0) return;

        var platformStats = new List<PlatformStat>(platforms.Length);

        foreach (var platform in platforms)
        {
            await ProcessPlatform(dbContext, platform, cancellationToken);
        }

        await dbContext.SaveChangesAsync(cancellationToken);
        await platformHub.PushPlatformsUpdates(platforms);
    }

    private async Task ProcessPlatform(ApplicationDbContext dbContext, Platform platform, CancellationToken ct)
    {
        var client = clientFactory.GetPlatformClient(platform.Address);

        try
        {
            var info = await client.GetPlatformInfoAsync(new Empty(), cancellationToken: ct);

            // Update platform
            platform.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networksCount: info.NetworksCount,
                volumesCount: info.VolumesCount,
                containers: info.Containers,
                containersRunning: info.ContainersRunning,
                containersPaused: info.ContainersPaused,
                containersStopped: info.ContainersStopped,
                images: info.Images,
                ncpu: info.Ncpu,
                memTotal: info.MemTotal,
                serverVersion: info.ServerVersion,
                agentVersion: info.AgentVersion,
                osType: info.OsType,
                osVersion: info.OsVersion,
                operatingSystem: info.OperatingSystem,
                driver: info.Driver);

            var stat = CreatePlatformStat(platform.Id, info);
            platform.Stats.Add(stat);
            dbContext.PlatformStats.Add(stat);
        }
        catch (RpcException ex)
        {
            logger.LogWarning(ex, "Error while getting system info from platform {PlatformId}", platform.Id);

            // Mark platform as offline
            platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
        }
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static PlatformStat CreatePlatformStat(Guid platformId, PlatformInfoMessage info)
    {
        return PlatformStat.Create(
            created: double.IsNaN(info.Created) ? 0 : info.Created,
            memoryUsage: double.IsNaN(info.MemoryUsage) ? 0 : info.MemoryUsage,
            cpuUsage: double.IsNaN(info.CpuUsage) ? 0 : info.CpuUsage,
            rxBytes: double.IsNaN(info.RxBytes) ? 0 : info.RxBytes,
            txBytes: double.IsNaN(info.TxBytes) ? 0 : info.TxBytes,
            platformId: platformId);
    }
}