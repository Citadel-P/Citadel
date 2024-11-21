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
                                    .AsSplitQuery()
                                    .Include(s => s.SystemInfo)
                                    .ThenInclude(s => s.SwarmInfo)
                                    .ThenInclude(s => s.RemoteManagers)
                                    .FirstOrDefaultAsync(s => s.SystemInfo.DaemonId == message.ID, cancellationToken);
        if (platform is null)
        {
            logger.LogError("No platform has been found for id={PlatformId}, please reconnect this platform", message.ID);
            return;
        }

        // Update db only if systemInfo record has changed
        if (!platform.SystemInfo.EqualsTo(message))
        {
            platform.SystemInfo.UpdateWith(message);
        }

        // Insert the platform stats
        await dbContext.PlatformStats.AddAsync(PlatformStat.Create(platform.Id, message.MemoryUsage, message.CpuUsage, message.CreatedAtUtc), cancellationToken);
        await dbContext.SaveChangesAsync(cancellationToken);

        // Notify client(s)
        await platformHub.SendPlatformUpdated(platform);
    }
}