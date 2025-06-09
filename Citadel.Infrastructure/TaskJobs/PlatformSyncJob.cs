using System.Threading.Channels;
using Infrastructure.Entities.Platforms;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Syncing platforms state.
/// </summary>
internal class PlatformSyncJob(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
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
                return;
            }

            var platformHub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();

            if (evt.IsOnLine)
            {
                var platformClient = clientFactory.GetPlatformClient(evt.Address);
                var containersClient = clientFactory.GetContainerClient(evt.Address);

                var platformInfo = await platformClient.ListPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

                // Update platform
                PlatformDescriptor? descriptor = null;
                if (platform.PlatformDescriptor is DockerPlatformDescriptor dockerDescriptor)
                {
                    descriptor = dockerDescriptor.Create(
                        daemonId: platformInfo.Id,
                        containerCount: platformInfo.ContainerCount,
                        containersRunning: platformInfo.ContainersRunning,
                        containersPaused: platformInfo.ContainersPaused,
                        containersStopped: platformInfo.ContainersStopped,
                        driver: platformInfo.Driver,
                        operatingSystem: platformInfo.OperatingSystem,
                        osVersion: platformInfo.OsVersion,
                        osType: platformInfo.OsType,
                        architecture: platformInfo.Architecture);
                }
                else if (platform.PlatformDescriptor is DockerSwarmPlatformDescriptor swarmDescriptor)
                {
                    // Todo
                }

                platform.PartialUpdate(
                    platformStatus: PlatformStatus.Online,
                    networkCount: platformInfo.NetworkCount,
                    volumeCount: platformInfo.VolumeCount,
                    imageCount: platformInfo.ImageCount,
                    memTotal: platformInfo.MemTotal,
                    serverVersion: platformInfo.ServerVersion,
                    agentVersion: platformInfo.AgentVersion,
                    descriptor: descriptor);
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
