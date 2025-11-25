using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class DockerDaemonEventJob(
    ILogger<DockerDaemonEventJob> logger,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IContainerEventBroadcaster containerEventBroadcaster,
    INotificationQueue notificationQueue,
    IDbWorkQueue dbWorkQueue) : BackgroundService
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

                await foreach (var reply in connectorFactory
                               .GetConnector(platform.Type)
                               .StreamDaemonEventAsync(command, cancellationToken))
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
                        if (reply.Action == "delete")
                        {
                            await OnImageDeleted(imageEvent, platform.Id, cancellationToken);
                        }
                    }
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                // normal shutdown
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while monitoring {Address}, retrying in 10s...", platform.Address);
                await Task.Delay(TimeSpan.FromSeconds(10), cancellationToken);
            }
        }
    }

    private ValueTask OnContainerCreated(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        if (eventInfo.Container is null) return ValueTask.CompletedTask;

        var dbItem = new ContainerCreatedWorkItem(
            eventInfo,
            platformId,
            notificationQueue,
            dockerDaemonHub,
            platformContainerCache,
            containerEventBroadcaster, logger);

        return dbWorkQueue.EnqueueAsync(dbItem, cancellationToken);
    }

    private ValueTask OnContainerUpdated(DaemonContainerEventInfo eventInfo, CancellationToken cancellationToken)
    {
        var item = new ContainerUpdatedWorkItem(eventInfo, notificationQueue, dockerDaemonHub, logger);

        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }

    private ValueTask OnContainerDestroyed(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var item = new ContainerDestroyedWorkItem(
            platformId,
            eventInfo,
            notificationQueue,
            dockerDaemonHub,
            platformContainerCache,
            containerEventBroadcaster, logger);

        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }

    private ValueTask OnImageDeleted(DaemonImageEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var item = new ImageDeletedWorkItem(platformId, eventInfo, notificationQueue, dockerDaemonHub, logger);

        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }
}
