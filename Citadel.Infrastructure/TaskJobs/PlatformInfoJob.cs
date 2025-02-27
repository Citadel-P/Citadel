using System.Collections.Concurrent;
using Citadel.Common;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class PlatformInfoJob(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<PlatformInfoJob> logger,
    IPlatformHubDispatcher platformHub) : IJob
{
    public static readonly JobKey JobKey = new(nameof(PlatformInfoJob), "SystemInfo");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var platforms = await dbContext.Platforms
                            .Include(s => s.SwarmInfo)
                            .ThenInclude(s => s.RemoteManagers)
                            .ToListAsync(context.CancellationToken);

        if (platforms.Count == 0) return;
        foreach (var platformData in await GetPlatformsData(platforms.Select(s => s.Address), context.CancellationToken)) 
        {
            var existing = platforms.First(s => s.Address == platformData.Address);
            existing.PartialUpdate(
                platformStatus: platformData.Status,
                networksCount: platformData.PlatformInfo?.NetworksCount,
                volumesCount: platformData.PlatformInfo?.VolumesCount,
                containers: platformData.PlatformInfo?.Containers,
                containersRunning: platformData.PlatformInfo?.ContainersRunning,
                containersPaused: platformData.PlatformInfo?.ContainersPaused,
                containersStopped: platformData.PlatformInfo?.ContainersStopped,
                images: platformData.PlatformInfo?.Images,
                ncpu: platformData.PlatformInfo?.Ncpu,
                memTotal: platformData.PlatformInfo?.MemTotal,
                serverVersion: platformData.PlatformInfo?.ServerVersion,
                agentVersion: platformData.PlatformInfo?.AgentVersion,
                osType: platformData.PlatformInfo?.OsType,
                osVersion: platformData.PlatformInfo?.OsVersion,
                operatingSystem: platformData.PlatformInfo?.OperatingSystem,
                driver: platformData.PlatformInfo?.Driver
                );

            if (platformData.Status == PlatformStatus.Online)
            {
                // Insert the platform stats
                var stat = PlatformStat.Create(
                    memoryUsage: double.IsNaN(platformData.PlatformInfo.MemoryUsage) ? 0 : platformData.PlatformInfo.MemoryUsage,
                    cpuUsage: double.IsNaN(platformData.PlatformInfo.CpuUsage) ? 0 : platformData.PlatformInfo.CpuUsage,
                    created: double.IsNaN(platformData.PlatformInfo.Created) ? 0 : platformData.PlatformInfo.Created,
                    rxBytes: double.IsNaN(platformData.PlatformInfo.RxBytes) ? 0 : platformData.PlatformInfo.RxBytes,
                    txBytes: double.IsNaN(platformData.PlatformInfo.TxBytes) ? 0 : platformData.PlatformInfo.TxBytes,
                    platformId: existing.Id);

                dbContext.PlatformStats.Add(stat);
            }
        }
        
        await dbContext.SaveChangesAsync(context.CancellationToken);
        // Notify client(s)
        await platformHub.PushPlatformsUpdates(platforms);
    }

    private async Task<IEnumerable<PlatformData>> GetPlatformsData(IEnumerable<string> addresses, CancellationToken cancellationToken) 
    {
        var platforms = new ConcurrentBag<PlatformData>();
        await Parallel.ForEachAsync(addresses, cancellationToken, async (address, cancellationToken) =>
        {
            var client = clientFactory.GetPlatformClient(address);
            try
            {
                var platformInfo = await client.GetPlatformInfoAsync(new Empty(), cancellationToken: cancellationToken);
                platforms.Add(new PlatformData(address, PlatformStatus.Online, platformInfo));
            }
            catch (RpcException ex)
            {
                logger.LogError(ex, "Error while getting system info from platform {PlatformAddress}", address);
                platforms.Add(new PlatformData(address, PlatformStatus.Offline));
            }
        });
        return platforms;
    }

    private record PlatformData(string Address, PlatformStatus Status, PlatformInfoMessage PlatformInfo = null);
}
