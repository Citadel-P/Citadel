using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class DockerDaemonEventJob(
    IServiceScopeFactory scopeFactory,
    ILogger<DockerDaemonEventJob> logger,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IContainerEventBroadcaster  containerEventBroadcaster) : BackgroundService
{
    private readonly ConcurrentDictionary<string, CancellationTokenSource> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();

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
                    if (reply is DaemonContainerEventInfo containerEvent)
                    {
                        switch (reply.Action)
                        {
                            case "create":
                                await OnContainerCreated(containerEvent, platform.Id, cancellationToken);
                                break;
                            case "destroy":
                                await OnContainerDestroyed(containerEvent, platform.Id, cancellationToken);
                                break;
                            default:
                                await OnContainerUpdated(containerEvent, cancellationToken);
                                break;
                        }
                    }
                    else if (reply is DaemonImageEventInfo imageEvent)
                    {
                        switch (reply.Action)
                        {
                            case "pull":
                            case "create":
                                await OnImageAddOrUpdate(imageEvent, platform.Id, cancellationToken);
                                break;
                            case "delete":
                                await OnImageDeleted(imageEvent, platform.Id, cancellationToken);
                                break;
                        }
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

    private async Task OnContainerCreated(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        if (eventInfo.Container != null)
        {
            var container = eventInfo.Container.Map(platformId);
            platformContainerCache.TryAddContainer(platformId, container.ContainerId, container.Id);

            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            await uow.Containers.AddAsync(container, cancellationToken);
            await uow.CommitAsync();

            await SendContainerEventChanges(container, eventInfo.Action);
        }
    }

    private async Task OnContainerUpdated(DaemonContainerEventInfo eventInfo, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var existing = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
        if (existing != null)
        {
            existing.PartialUpdate(state: eventInfo.Container?.State, ports: eventInfo.Container?.Ports);
            await uow.Containers.UpdateAsync(existing, cancellationToken);
            await uow.CommitAsync();
            await SendContainerEventChanges(existing, eventInfo.Action);
        }
    }

    private async Task OnContainerDestroyed(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var existingDestroy = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
        if (existingDestroy != null)
        {
            platformContainerCache.TryRemoveContainer(platformId, existingDestroy.ContainerId);

            await uow.Containers.DeleteAsync([existingDestroy.Id], cancellationToken);
            await uow.CommitAsync();

            await SendContainerEventChanges(existingDestroy, eventInfo.Action);
        }
    }

    private async Task OnImageAddOrUpdate(DaemonImageEventInfo imageEvent, Guid id, CancellationToken cancellationToken)
    {
        if (imageEvent.Image != null)
        {
            var image = imageEvent.Image.Map(id);
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            await uow.Images.AddOrUpdateAsync(image, cancellationToken);
            await uow.CommitAsync();

            await SendImageEventChanges(image, imageEvent.Action);
        }
    }


    private async Task OnImageDeleted(DaemonImageEventInfo eventInfo, Guid id, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var existing = await uow.Images.GetByImageIdAsync(eventInfo.ImageId, cancellationToken);
        if (existing != null)
        {
            await uow.Images.DeleteAsync([existing.Id], cancellationToken);
            await uow.CommitAsync();

            await SendImageEventChanges(existing, eventInfo.Action);
        }
    }

    private async Task SendContainerEventChanges(Container container, string action)
    {
        try
        {
            await dockerDaemonHub.SendContainerEvent(container, action);
            await containerEventBroadcaster.PublishAsync(new ContainerEvent(container.PlatformId, container.ContainerId, action));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to notify clients about container stats for platform {PlatformId}", container.PlatformId);
        }
    }

    private async Task SendImageEventChanges(Image image, string action)
    {
        try
        {
            await dockerDaemonHub.SendImageEvent(image, action);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to notify clients about image event for platform {PlatformId}", image.PlatformId);
        }
    }
}
