using System.Collections.Concurrent;
using Agent.Server.Containers;
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
            .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
            .Include(s => s.SwarmInfo)
            .ThenInclude(s => s.RemoteManagers)
            .ToListAsync(context.CancellationToken);

        if (platforms.Count == 0) return;

        var platformMap = platforms.ToDictionary(p => p.Address);
        var platformDataList = await GetPlatformsData(platformMap.Keys, context.CancellationToken);

        foreach (var platformData in platformDataList)
        {
            if (!platformMap.TryGetValue(platformData.Address, out var existing))
                continue;

            var info = platformData.PlatformInfo;

            existing.PartialUpdate(
                platformStatus: platformData.Status,
                networksCount: info?.NetworksCount,
                volumesCount: info?.VolumesCount,
                containers: info?.Containers,
                containersRunning: info?.ContainersRunning,
                containersPaused: info?.ContainersPaused,
                containersStopped: info?.ContainersStopped,
                images: info?.Images,
                ncpu: info?.Ncpu,
                memTotal: info?.MemTotal,
                serverVersion: info?.ServerVersion,
                agentVersion: info?.AgentVersion,
                osType: info?.OsType,
                osVersion: info?.OsVersion,
                operatingSystem: info?.OperatingSystem,
                driver: info?.Driver);

            if (platformData.Status == PlatformStatus.Online && info != null)
            {
                // Insert the platform stats
                var stat = PlatformStat.Create(
                    memoryUsage: double.IsNaN(info.MemoryUsage) ? 0 : info.MemoryUsage,
                    cpuUsage: double.IsNaN(info.CpuUsage) ? 0 : info.CpuUsage,
                    created: double.IsNaN(info.Created) ? 0 : info.Created,
                    rxBytes: double.IsNaN(info.RxBytes) ? 0 : info.RxBytes,
                    txBytes: double.IsNaN(info.TxBytes) ? 0 : info.TxBytes,
                    platformId: existing.Id);

                dbContext.PlatformStats.Add(stat);
            }
        }

        await dbContext.SaveChangesAsync(context.CancellationToken);

        platforms.Sort((a, b) => string.Compare(b.Name, a.Name, StringComparison.Ordinal));
        // Notify client(s)
        await platformHub.PushPlatformsUpdates(platforms);
    }

    private async Task<IEnumerable<PlatformData>> GetPlatformsData(IEnumerable<string> addresses, CancellationToken cancellationToken)
    {
        var result = new ConcurrentBag<PlatformData>();

        await Parallel.ForEachAsync(addresses, cancellationToken, async (address, ct) =>
        {
            var client = clientFactory.GetPlatformClient(address);
            try
            {
                var info = await client.GetPlatformInfoAsync(new Empty(), cancellationToken: ct);
                result.Add(new PlatformData(address, PlatformStatus.Online, info));
            }
            catch (RpcException ex)
            {
                logger.LogWarning(ex, "Error while getting system info from platform {PlatformAddress}", address);
                result.Add(new PlatformData(address, PlatformStatus.Offline));
            }
        });

        return result;
    }

    private readonly record struct PlatformData(string Address, PlatformStatus Status, PlatformInfoMessage PlatformInfo = null);
}
