using System.Collections.Concurrent;
using System.Threading.Channels;
using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DockerDaemonEventJob(
    IServiceScopeFactory scopeFactory,
    ILogger<DockerDaemonEventJob> logger,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IPlatformConnector> connectorFactory,
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

        _ = MonitorStream(platform, cts.Token);
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
                var command = new StreamDaemonEventCommand(platform.Address);
                await foreach (var reply in connectorFactory.GetConnector(platform.Type).StreamDaemonEventAsync(command, cancellationToken))
                {
                    if (reply.Type != ContainerEventType.Container)
                        continue;


                    switch (reply.Action)
                    {
                        case "create":
                            await HandleContainerCreateEvent(reply, platform.Id, cancellationToken);
                            break;
                        case "destroy":
                            await HandleContainerDestroyEvent(reply, platform.Id, cancellationToken);
                            break;
                        default:
                            await HandleContainerUpdateEvent(reply, cancellationToken);
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

    private async Task HandleContainerCreateEvent(DaemonEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        if (eventInfo.Container != null)
        {
            var container = eventInfo.Container.Map(platformId);

            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            await uow.Containers.AddAsync(container, cancellationToken);
            await uow.CommitAsync();

            await SendContainerEventChanges(container, eventInfo.Action);
            platformContainerCache.TryAddContainer(platformId, container.ContainerId, container.Id);
        }
    }

    private async Task HandleContainerUpdateEvent(DaemonEventInfo eventInfo, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var existing = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
        if (existing != null)
        {
            existing.PartialUpdate(state: eventInfo.Container?.State);
            await uow.Containers.UpdateContainersStateAsync([existing.Id], existing.State, cancellationToken);
            await uow.CommitAsync();
            await SendContainerEventChanges(existing, eventInfo.Action);
        }
    }

    private async Task HandleContainerDestroyEvent(DaemonEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var existingDestroy = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
        if (existingDestroy != null)
        {
            await uow.Containers.DeleteAsync([existingDestroy.Id], cancellationToken);
            await uow.CommitAsync();

            await SendContainerEventChanges(existingDestroy, eventInfo.Action);
            platformContainerCache.TryRemoveContainer(platformId, existingDestroy.ContainerId);
        }
    }

    private async Task SendContainerEventChanges(Container container, string action)
    {
        try
        {
            await dockerDaemonHub.SendContainerEvent(container, action);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to notify clients about containers stats for platform {PlatformId}", container.PlatformId);
        }
    }
}
