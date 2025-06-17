using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Services;
using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DockerDaemonEventJob(
    ILogger<DockerDaemonEventJob> logger,
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster) : BackgroundService
{
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.Register();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await foreach (var platform in platformHealthReader.ReadAllAsync(cancellationToken))
        {
            if (platform.IsOnLine)
            {
                StartMonitoringPlatform(platform, cancellationToken);
            }
            else
            {
                StopMonitoringPlatform(platform.Address);
            }
        }
    }

    public void StartMonitoringPlatform(PlatformHealth platform, CancellationToken cancellationToken)
    {
        if (runningStreams.ContainsKey(platform.Address))
        {
            logger.LogWarning("Monitoring daemon events for {Address} is already running.", platform.Address);
            return;
        }

        var cts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        runningStreams[platform.Address] = cts;

        _ = Task.Run(() => MonitorStream(platform, cts.Token), cts.Token);
    }

    public void StopMonitoringPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var cts))
        {
            logger.LogInformation("Aborting monitoring daemon events for {Address}", address);
            cts.Cancel();
        }
    }

    private async Task MonitorStream(PlatformHealth platform, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                using var scope = scopeFactory.CreateAsyncScope();
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var containerHub = scope.ServiceProvider.GetRequiredService<IContainerHubDispatcher>();

                var command = new StreamDaemonEventCommand
                (
                    PlatformId: platform.Id,
                    PlatformAddress: platform.Address
                );
                await foreach (var reply in connectorFactory.GetConnector(platform.Type).StreamDaemonEventAsync(command, cancellationToken))
                {
                    if (reply.Type != ContainerEventType.Container)
                        continue;


                    switch (reply.Action)
                    {
                        case "create":
                            if (reply.Container != null)
                            {
                                var container = reply.Container;
                                uow.Containers.Add(container);
                                await uow.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(container, reply.Action);
                                platformContainerCache.TryAddContainer(platform.Id, container.ContainerId, container.Id);
                            }
                           
                            break;
                        case "destroy":
                            var existingDestroy = await uow.Containers.Query().FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existingDestroy != null)
                            {
                                uow.Containers.Remove(existingDestroy);
                                await uow.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(existingDestroy, reply.Action);
                                platformContainerCache.TryRemoveContainer(platform.Id, existingDestroy.ContainerId);
                            }
                            break;
                        default:
                            var existing = await uow.Containers.Query().FirstOrDefaultAsync(c => c.ContainerId == reply.ContainerId, cancellationToken);
                            if (existing != null)
                            {
                                var state = reply.Action switch
                                {
                                    "stop" => "exited",
                                    "start" => "running",
                                    "pause" => "paused",
                                    "restart" => "restarting",
                                    _ => throw new NotImplementedException()
                                };

                                existing.PartialUpdate(state: reply.Container?.State);
                                await uow.SaveChangesAsync(cancellationToken);
                                await containerHub.SendContainerEvent(existing, reply.Action);
                            }
                            break;
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while monitoring {Address}, retrying in 10s...", platform.Address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
        }
    }
}
