using Application.Services.Abstractions;
using Gplatform;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal class PlatformService(
    ApplicationDbContext dbContext,
    IPlatformHubDispatcher platformHub,
    ILogger<PlatformService> logger) : IPlatformService
{
    public async Task OnSystemInfoMessage(SystemInfoMessage message, CancellationToken cancellationToken)
    {
        // Get the platform from db
        Platform platform = await dbContext.Platforms
                                    .Include(s => s.SystemInfo)
                                    .ThenInclude(s => s.SwarmInfo)
                                    .ThenInclude(s => s.RemoteManagers)
                                    .FirstOrDefaultAsync(s => s.SystemInfo.DaemonId == message.Id, cancellationToken);
        if (platform is null)
        {
            logger.LogError("No platform has been found for id={PltaformId}, please reconnect this platform", message.Id);
            return;
        }

        platform.SystemInfo.PartialUpdate(
            networksCount: message.NetworksCount,
            volumesCount: message.VolumesCount,
            containers: message.Containers,
            containersRunning: message.ContainersRunning,
            containersPaused: message.ContainersPaused,
            containersStopped: message.ContainersStopped,
        images: message.Images,
        ncpu: message.Ncpu,
            memTotal: message.MemTotal,
            serverVersion: message.ServerVersion,
            agentVersion: message.AgentVersion,
            osType: message.OsType,
            osVersion: message.OsVersion,
            operatingSystem: message.OperatingSystem,
        driver: message.Driver
            );

        // Insert the platform stats
        var stat = PlatformStat.Create(
            memoryUsage: message.MemoryUsage,
            cpuUsage: message.CpuUsage,
            created: message.Created,
            rxBytes: message.RxBytes,
            txBytes: message.TxBytes,
            platformId: platform.Id);

        dbContext.PlatformStats.Add(stat);
        await dbContext.SaveChangesAsync(cancellationToken);

        // Notify client(s)
        await platformHub.SendPlatformUpdated(platform);
    }
}
