using System.Runtime.CompilerServices;
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
    private static readonly ParallelOptions _parallelOptions = new()
    {
        MaxDegreeOfParallelism = Environment.ProcessorCount
    };

    public async ValueTask Execute(IJobExecutionContext context)
    {
        var platforms = await dbContext.Platforms
            .OrderByDescending(s => s.Name)
            .ToArrayAsync(context.CancellationToken);

        if (platforms.Length == 0) return;

        // Preallocate arrays with known size
        var updates = new PlatformUpdate[platforms.Length];
        var platformStats = new List<PlatformStat>(platforms.Length);

        // Process platforms in parallel while minimizing allocations
        _parallelOptions.CancellationToken = context.CancellationToken;
        await Parallel.ForEachAsync(new ArraySegment<Platform>(platforms), _parallelOptions, ProcessPlatform);

        // Save changes to the database in one batch
        if (platformStats.Count > 0)
        {
            dbContext.PlatformStats.AddRange(platformStats);
        }

        await dbContext.SaveChangesAsync(context.CancellationToken);
        await platformHub.PushPlatformsUpdates(platforms);

        // Local async function captures outer variables but avoids lambda allocation
        async ValueTask ProcessPlatform(Platform platform, CancellationToken ct)
        {
            int platformIndex = Array.IndexOf(platforms, platform);
            var client = clientFactory.GetPlatformClient(platform.Address);

            try
            {
                var info = await client.GetPlatformInfoAsync(new Empty(), cancellationToken: ct);
                updates[platformIndex] = new PlatformUpdate(platform.Address, PlatformStatus.Online, info);

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

                platformStats.Add(CreatePlatformStat(platform.Id, info));
            }
            catch (RpcException ex)
            {
                logger.LogWarning(ex, "Error while getting system info from platform {PlatformId}", platform.Id);
                updates[platformIndex] = new PlatformUpdate(platform.Address, PlatformStatus.Offline);

                // Mark platform as offline
                platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
            }
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

    private readonly struct PlatformUpdate(string address, PlatformStatus status, PlatformInfoMessage info = null)
    {
        public string Address { get; } = address;
        public PlatformStatus Status { get; } = status;
        public PlatformInfoMessage Info { get; } = info;
    }
}