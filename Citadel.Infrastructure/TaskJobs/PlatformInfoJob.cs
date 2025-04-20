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
        // Fetch platforms and create a dictionary directly
        var platforms = await dbContext.Platforms
            .OrderByDescending(s => s.Name)
            .ToDictionaryAsync(p => p.Address, p => p, context.CancellationToken);

        if (platforms.Count == 0) return;

        // Precompute platform index mapping for updates
        var platformIndexMap = platforms.Keys
            .Select((address, index) => new { address, index })
            .ToDictionary(x => x.address, x => x.index);

        var updates = new PlatformUpdate[platforms.Count];
        var platformStats = new List<PlatformStat>(platforms.Count);

        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = context.CancellationToken
        };

        // Process platforms in parallel
        await Parallel.ForEachAsync(platforms, parallelOptions, async (platformEntry, ct) =>
        {
            var (address, platform) = platformEntry;
            var client = clientFactory.GetPlatformClient(platform.Address);

            try
            {
                var info = await client.GetPlatformInfoAsync(new Empty(), cancellationToken: ct);
                updates[platformIndexMap[address]] = new PlatformUpdate(platform.Address, PlatformStatus.Online, info);
            }
            catch (RpcException ex)
            {
                logger.LogWarning(ex, "Error while getting system info from platform {PlatformId}", platform.Id);
                updates[platformIndexMap[address]] = new PlatformUpdate(platform.Address, PlatformStatus.Offline);
            }
        });

        // Update platforms and prepare statistics
        foreach (var update in updates)
        {
            var platform = platforms[update.Address];
            var info = update.Info;

            if (update.Status == PlatformStatus.Online && info is not null)
            {
                platform.PartialUpdate(
                    platformStatus: update.Status,
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

                platformStats.Add(PlatformStat.Create(
                    memoryUsage: double.IsNaN(info.MemoryUsage) ? 0 : info.MemoryUsage,
                    cpuUsage: double.IsNaN(info.CpuUsage) ? 0 : info.CpuUsage,
                    created: double.IsNaN(info.Created) ? 0 : info.Created,
                    rxBytes: double.IsNaN(info.RxBytes) ? 0 : info.RxBytes,
                    txBytes: double.IsNaN(info.TxBytes) ? 0 : info.TxBytes,
                    platformId: platform.Id));
            }
            else
            {
                // Mark platform as offline
                platform.PartialUpdate(platformStatus: update.Status);
            }
        }

        // Batch save changes to the database
        if (platformStats.Count > 0)
        {
            dbContext.PlatformStats.AddRange(platformStats);
        }

        await dbContext.SaveChangesAsync(context.CancellationToken);

        // Push platform updates to the hub
        await platformHub.PushPlatformsUpdates(platforms.Values);
    }

    private readonly struct PlatformUpdate(string address, PlatformStatus status, PlatformInfoMessage info = null)
    {
        public string Address { get; } = address;
        public PlatformStatus Status { get; } = status;
        public PlatformInfoMessage Info { get; } = info;
    }
}
