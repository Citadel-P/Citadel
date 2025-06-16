using System.Threading.Channels;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Infrastructure.EntityFramework;
using Application.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Syncing platforms state.
/// </summary>
internal class PlatformSyncJob(
    IServiceScopeFactory scopeFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var evt in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            await SyncPlatform(evt, cancellationToken);
        }
    }

    private async Task SyncPlatform(PlatformHealth evt, CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            var platform = await db.Platforms.FirstOrDefaultAsync(s => s.Address == evt.Address, cancellationToken);
            if (platform == null)
            {
                logger.LogError("Platform with address {Address} not found for synchronization.", evt.Address);
                return;
            }

            var platformHub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();

            if (evt.IsOnLine)
            {
                var param = new GetPlatformCommand
                (
                    PlatformName: platform.Name,
                    PlatformAddress: platform.Address
                );
                var platformResult = await connectorFactory.GetConnector(evt.Type).GetPlatformAsync(param, cancellationToken);
                if (!platformResult.IsSuccess(out var platformInfo, out var error))
                {
                    logger.LogError("Failed to get platform info for {Address}: {Error}", platform.Address, error?.Message);
                    return;
                }
                // Update platform
               
                platform.PartialUpdate(
                    platformStatus: PlatformStatus.Online,
                    networkCount: platformInfo.NetworkCount,
                    volumeCount: platformInfo.VolumeCount,
                    imageCount: platformInfo.ImageCount,
                    memTotal: platformInfo.MemTotal,
                    serverVersion: platformInfo.ServerVersion,
                    agentVersion: platformInfo.AgentVersion,
                    descriptor: platformInfo.PlatformDescriptor);
            }
            else
            {
                platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
            }

            await db.SaveChangesAsync(cancellationToken);
            await platformHub.PushPlatformUpdate(platform);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while synchronizing platform {Address}", evt.Address);
        }
    }
}
