using Contracts.Broker.Models;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Infrastructure.Services.Abstractions;
using Infrastructure.EntityFramework;
using Application.Services.Abstractions;

namespace Application.Services;

internal sealed class PlatformService(
    ApplicationDbContext dbContext,
    IPlatformHubDispatcher platformHub,
    ILogger<PlatformService> logger) : IPlatformService
{
    /// <inheritdoc/>
    public async Task OnSystemInfoMessage(SystemInfoMessage message, CancellationToken cancellationToken = default)
    {
        // Get the platform from db
        Platform platform = await dbContext.Platforms
                                    .Include(s => s.SystemInfo)
                                    .ThenInclude(s => s.SwarmInfo)
                                    .ThenInclude(s => s.RemoteManagers)
                                    .FirstOrDefaultAsync(s => s.SystemInfo.DaemonId == message.ID, cancellationToken);
        if (platform is null)
        {
            logger.LogError("No platform has been found for id={PlatformId}, please reconnect this platform", message.ID);
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
            ncpu: message.NCPU,
            memTotal: message.MemTotal,
            serverVersion: message.ServerVersion,
            agentVersion: message.AgentVersion,
            osType: message.OSType,
            osVersion: message.OSVersion,
            operatingSystem: message.OperatingSystem,
            driver: message.Driver
            );

        // Insert the platform stats
        var stat = PlatformStat.Create(
            memoryUsage: message.MemoryUsage,
            cpuUsage: message.CpuUsage,
            created: message.Created,
            rxBytes: message.RxBytes.Value,
            txBytes: message.TxBytes.Value,
            platformId: platform.Id);

        dbContext.PlatformStats.Add(stat);
        await dbContext.SaveChangesAsync(cancellationToken);

        // Notify client(s)
        await platformHub.SendPlatformUpdated(platform);
    }
}