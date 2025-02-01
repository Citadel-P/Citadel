using Citadel.Common;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Quartz;

namespace Infrastructure.TaskJobs;

internal class SystemInfoJob(
    ICacheService cacheService, 
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<SystemInfoJob> logger,
    IPlatformHubDispatcher platformHub) : IJob
{
    public static readonly JobKey JobKey = new(nameof(SystemInfoJob), "SystemInfo");

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var addresses = await cacheService.GetClientsAddresses(context.CancellationToken);
        if (!addresses.Any()) return;

        var systemsInfo = new List<SystemInfoMessage>();
        await Parallel.ForEachAsync(addresses, async (address, token) =>
        {
            var client = clientFactory.GetPlatformClient(address);
            try
            {
                var systemInfo = await client.GetSystemInfoAsync(new Empty(), cancellationToken: token);
                systemsInfo.Add(systemInfo);
            }
            catch (RpcException ex)
            {
                logger.LogError(ex, "Error while getting system info from platform {PlatformAddress}", address);
            }
        });

        var platforms = await dbContext.Platforms
                            .Include(s => s.SystemInfo)
                            .ThenInclude(s => s.SwarmInfo)
                            .ThenInclude(s => s.RemoteManagers)
                            .Where(s => systemsInfo.Select(s => s.Id).Contains(s.SystemInfo.DaemonId))
                            .ToListAsync(context.CancellationToken);

        foreach (var platform in platforms)
        {
            var systemInfo = systemsInfo.FirstOrDefault(s => s.Id == platform.SystemInfo.DaemonId);
            if (systemInfo is null)
            {
                logger.LogError("No system info has been found for platform {PlatformId}", platform.Id);
                continue;
            }
            platform.SystemInfo.PartialUpdate(
                networksCount: systemInfo.NetworksCount,
                volumesCount: systemInfo.VolumesCount,
                containers: systemInfo.Containers,
                containersRunning: systemInfo.ContainersRunning,
                containersPaused: systemInfo.ContainersPaused,
                containersStopped: systemInfo.ContainersStopped,
                images: systemInfo.Images,
                ncpu: systemInfo.Ncpu,
                memTotal: systemInfo.MemTotal,
                serverVersion: systemInfo.ServerVersion,
                agentVersion: systemInfo.AgentVersion,
                osType: systemInfo.OsType,
                osVersion: systemInfo.OsVersion,
                operatingSystem: systemInfo.OperatingSystem,
                driver: systemInfo.Driver
                );
            // Insert the platform stats
            var stat = PlatformStat.Create(
                memoryUsage: systemInfo.MemoryUsage,
                cpuUsage: systemInfo.CpuUsage,
                created: systemInfo.Created,
                rxBytes: systemInfo.RxBytes,
                txBytes: systemInfo.TxBytes,
                platformId: platform.Id);

            dbContext.PlatformStats.Add(stat);    
        }

        await dbContext.SaveChangesAsync(context.CancellationToken);
        // Notify client(s)
        await platformHub.PushPlatformsUpdates(platforms);
    }
}
