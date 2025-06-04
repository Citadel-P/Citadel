using System.Threading.Channels;
using Agent.Server.Containers;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Background service that trigger the monitors of gRPC services and synchronizes platform and container information.
/// </summary>
internal class SyncTriggerService(
    IGrpcClientFactory clientFactory,
    IServiceScopeFactory scopeFactory,
    IGrpcHealthMonitorJob grpcHealthMonitorJob,
    ChannelReader<GrpcServiceHealth> channelReader,
    ILogger<SyncTriggerService> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        string[]? addresses;
        // Create a scope and dbContext only for the initial fetch
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            addresses = await dbContext.Platforms.AsNoTracking().Select(s => s.Address).ToArrayAsync(cancellationToken);
        }

        if (addresses != null && addresses.Length > 0)
        {
            grpcHealthMonitorJob.TrackAddress(addresses);
        }

        await foreach (var evt in channelReader.ReadAllAsync(cancellationToken))
        {
            await SyncPlatform(evt, cancellationToken);
        }
    }

    private async Task SyncPlatform(GrpcServiceHealth evt, CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
            var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Address == evt.Address, cancellationToken);
            if (platform == null)
            {
                return;
            }
            ContainerInfo[]? containers;
            var platformHub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();
            var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

            if (evt.IsOnLine)
            {
                var platformClient = clientFactory.GetPlatformClient(evt.Address);
                var containersClient = clientFactory.GetContainerClient(evt.Address);

                var platformInfoTsk = platformClient.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
                var containersTsk = containersClient.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: cancellationToken);

                var platformInfo = await platformInfoTsk;
                var containersReply = await containersTsk;
                containers = [.. containersReply.Containers.Select(s => s.Value.Map(platform.Id, DateTimeOffset.UtcNow.ToUnixTimeSeconds()))];

                // Update or add containers
                var existingContainers = await dbContext.ContainersInfo
                    .Where(c => c.PlatformId == platform.Id)
                    .ToDictionaryAsync(c => c.ContainerId, cancellationToken);

                foreach (var container in containers)
                {
                    if (existingContainers.TryGetValue(container.ContainerId, out var existing))
                    {
                        existing.PartialUpdate(
                            name: container.Name,
                            image: container.Image,
                            state: container.State,
                            stack: container.Stack,
                            created: container.Created,
                            ports: container.Ports
                        );
                    }
                    else
                    {
                        dbContext.ContainersInfo.Add(container);
                    }
                }

                // Remove stale containers
                var stale = existingContainers.Where(c => !containers.Any(s => s.ContainerId == c.Key)).Select(s => s.Value).ToArray();
                dbContext.ContainersInfo.RemoveRange(stale);

                // Update platform
                if (platform.PlatformDescriptor is DockerPlatformDescriptor dockerPlatform)
                {
                    dockerPlatform.PartialUpdate(
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
                else if (platform.PlatformDescriptor is DockerSwarmPlatformDescriptor dockerSwarmPlatform)
                {
                    // Todo: implement DockerSwarmPlatform configuration update
                }

                platform.PartialUpdate(
                    platformStatus: PlatformStatus.Online,
                    networkCount: platformInfo.NetworkCount,
                    volumeCount: platformInfo.VolumeCount,
                    containersRunning: platformInfo.ContainersRunning,
                    containersPaused: platformInfo.ContainersPaused,
                    containersStopped: platformInfo.ContainersStopped,
                    imageCount: platformInfo.ImageCount,
                    memTotal: platformInfo.MemTotal,
                    serverVersion: platformInfo.ServerVersion,
                    agentVersion: platformInfo.AgentVersion);
            }
            else
            {
                platform.PartialUpdate(platformStatus: PlatformStatus.Offline);
                containers = await dbContext.ContainersInfo.Where(c => c.PlatformId == platform.Id).ToArrayAsync(cancellationToken);
                foreach (var container in containers)
                {
                    container.PartialUpdate(state: ContainerStateStatus.Offline);
                }
            }

            await dbContext.SaveChangesAsync(cancellationToken);
            await platformHub.PushPlatformUpdate(platform);
            if (containers != null) await containerHub.SendContainersInfo(containers);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error while synchronizing platform {Address}", evt.Address);
        }
    }
}
