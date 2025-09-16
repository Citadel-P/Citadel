using System.Threading.Channels;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Synchronizes platform state.
/// </summary>
internal class PlatformSyncJob(
    IServiceScopeFactory scopeFactory,
    IPlatformStreamManager platformStreamManager,
    IPlatformContainerCache platformContainerCache,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    ILogger<PlatformSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(12);
    
    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{PlatformSyncJob} started. Running every {Hours} hours.", nameof(PlatformSyncJob), SyncInterval.TotalHours);

        // Event-driven sync starts immediately
        var eventDrivenTask = RunEventDrivenSync(cancellationToken);

        // Periodic sync starts with jitter
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    /// <summary>
    /// Reacts to platform health events and syncs platforms as they change state.
    /// </summary>
    private async Task RunEventDrivenSync(CancellationToken cancellationToken)
    {
        await foreach (var evt in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            await SyncPlatform(evt, cancellationToken);
        }
    }

    /// <summary>
    /// Periodically syncs all platforms every x hours.
    /// </summary>
    private async Task RunPeriodicSync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await SyncAllPlatforms(cancellationToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during periodic container synchronization.");
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    /// <summary>
    /// Fetches all platforms from the database and syncs them.
    /// </summary>
    private async Task SyncAllPlatforms(CancellationToken cancellationToken)
    {
        if (platformContainerCache.TryGetCacheEntries(out var platforms, out var _))
        {
            foreach (var platform in platforms)
            {
                if (cancellationToken.IsCancellationRequested) break;

                try
                {
                    var platformEvent = new PlatformHealth(
                        Id: platform.Id,
                        Type: platform.ConnectorType,
                        Address: platform.Address,
                        IsOnLine: true
                    );

                    await SyncPlatform(platformEvent, cancellationToken);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Error syncing containers for platform {PlatformId}", platform.Id);
                }
            }
        }
    }

    /// <summary>
    /// Core logic to synchronize containers for a single platform.
    /// </summary>
    private async Task SyncPlatform(PlatformHealth evt, CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = await uow.Platforms.GetByIdAsync(evt.Id, cancellationToken);
            if (platform == null)
            {
                logger.LogError("Platform with address {Address} not found for synchronization.", evt.Address);
                return;
            }

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
                    cpuCount: platformInfo.CpuCount,
                    descriptor: platformInfo.Descriptor);
            }
            else
            {
                platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
            }

            await uow.Platforms.UpdatePlatformAsync(platform, cancellationToken);
            await uow.CommitAsync();

            await platformStreamManager.PushPlatformUpdate(platform);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while synchronizing platform {Address}", evt.Address);
        }
    }
}
