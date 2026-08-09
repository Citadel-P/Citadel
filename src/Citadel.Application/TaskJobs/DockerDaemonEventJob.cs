using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Collections.Concurrent;
using System.Threading.Channels;
using static Application.Services.PullImageService;

namespace Application.TaskJobs;

internal sealed class DockerDaemonEventJob(
    ILogger<DockerDaemonEventJob> logger,
    IActivityStreamManager activityHub,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IPlatformConnector> connectorFactory,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    IContainerEventBroadcaster containerEventBroadcaster,
    INotificationQueue notificationQueue,
    ChannelWriter<UnmanagedContainerAlertRequest> unmanagedContainerAlertWriter,
    IStackStreamManager stackHub,
    IImageStreamManager imageStream,
    IDeploymentStreamManager deploymentHub,
    ISwarmReconciliationCoordinator swarmReconciliationCoordinator,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    SwarmTaskContainerPruner swarmTaskContainerPruner,
    IDbWorkQueue dbWorkQueue) : BackgroundService
{
    private static readonly TimeSpan ReconnectDelay = TimeSpan.FromSeconds(10);
    private readonly ConcurrentDictionary<string, DaemonMonitorRegistration> runningStreams = new();
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var platform in platformHealthReader.ReadAllAsync(cancellationToken))
            {
                if (platform.IsOnLine && platform.IsValidated)
                    StartMonitoringPlatform(platform, cancellationToken);
                else
                    StopMonitoringPlatform(platform.Address);
            }
        }
        finally
        {
            platformHealthBroadCaster.RemoveSubscriber(platformHealthReader);
            var registrations = runningStreams.Values.ToArray();
            foreach (var registration in registrations)
                StopMonitoringPlatform(registration.Address);

            await Task.WhenAll(registrations.Select(static registration => registration.Task));
        }
    }

    public void StartMonitoringPlatform(PlatformHealth platform, CancellationToken cancellationToken)
    {
        var registration = new DaemonMonitorRegistration(
            platform.Address,
            CancellationTokenSource.CreateLinkedTokenSource(cancellationToken));
        if (!runningStreams.TryAdd(platform.Address, registration))
        {
            registration.Dispose();
            logger.LogWarning("Monitoring daemon events for {Address} is already running.", platform.Address);
            return;
        }

        registration.Task = MonitorStreamOwnedAsync(platform, registration);
    }

    public void StopMonitoringPlatform(string address)
    {
        if (runningStreams.TryRemove(address, out var registration))
        {
            logger.LogInformation("Aborting monitoring daemon events for {Address}", address);
            registration.Cancel();
        }
    }

    private async Task MonitorStreamOwnedAsync(
        PlatformHealth platform,
        DaemonMonitorRegistration registration)
    {
        var cancellationToken = registration.Token;
        try
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
                        await swarmReconciliationCoordinator.NotifyDaemonEventAsync(
                            platform.Id,
                            reply,
                            cancellationToken);

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
                                    if (containerEvent.Container is
                                        {
                                            IsSwarmTask: true,
                                            State: ContainerStateStatus.Exited or ContainerStateStatus.Dead
                                        } historicalTask)
                                    {
                                        await swarmTaskContainerPruner.PruneAsync(
                                            platform,
                                            containerConnectorFactory.GetConnector(platform.Type),
                                            [historicalTask],
                                            cancellationToken);
                                    }
                                    break;
                            }
                        }
                        else if (reply is DaemonImageEventInfo imageEvent)
                        {
                            switch (reply.Action)
                            {
                                case "create":
                                case "pull":
                                    await OnImageCreated(imageEvent, platform.Id, cancellationToken);
                                    break;
                                case "delete":
                                    await OnImageDeleted(imageEvent, platform.Id, cancellationToken);
                                    break;

                                default: break;
                            }
                        }
                        else if (reply is DaemonVolumeEventInfo volumeEvent)
                        {
                            await OnVolumeEvent(volumeEvent, platform.Id, cancellationToken);
                        }
                        else if (reply is DaemonNetworkEventInfo networkEvent)
                        {
                            await OnNetworkEvent(networkEvent, platform.Id, cancellationToken);
                        }
                    }

                    logger.LogWarning(
                        "Daemon event stream for {Address} completed; retrying in {Delay}.",
                        platform.Address,
                        ReconnectDelay);
                }
                catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
                {
                    break;
                }
                catch (Exception ex)
                {
                    logger.LogError(
                        ex,
                        "Error while monitoring {Address}, retrying in {Delay}.",
                        platform.Address,
                        ReconnectDelay);
                }

                await Task.Delay(ReconnectDelay, cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        finally
        {
            if (runningStreams.TryGetValue(platform.Address, out var current)
                && ReferenceEquals(current, registration))
            {
                runningStreams.TryRemove(platform.Address, out _);
            }

            registration.Dispose();
        }
    }

    private sealed class DaemonMonitorRegistration(string address, CancellationTokenSource cancellation) : IDisposable
    {
        public string Address { get; } = address;
        public CancellationToken Token => cancellation.Token;
        public Task Task { get; set; } = Task.CompletedTask;

        public void Cancel()
        {
            try { cancellation.Cancel(); } catch { }
        }

        public void Dispose() => cancellation.Dispose();
    }

    private ValueTask OnContainerCreated(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        if (eventInfo.Container is null) return ValueTask.CompletedTask;

        var dbItem = new ContainerCreatedWorkItem(
            eventInfo,
            platformId,
            notificationQueue,
            unmanagedContainerAlertWriter,
            dockerDaemonHub,
            platformContainerCache,
            containerEventBroadcaster, logger);

        return dbWorkQueue.EnqueueAsync(dbItem, cancellationToken);
    }

    private ValueTask OnContainerUpdated(DaemonContainerEventInfo eventInfo, CancellationToken cancellationToken)
    {
        var item = new ContainerUpdatedWorkItem(eventInfo, notificationQueue, activityHub, deploymentHub, dockerDaemonHub, containerEventBroadcaster, stackHub, logger);

        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }

    private ValueTask OnContainerDestroyed(DaemonContainerEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var item = new ContainerDestroyedWorkItem(
            platformId,
            eventInfo,
            notificationQueue,
            stackHub,
            activityHub,
            deploymentHub,
            dockerDaemonHub,
            platformContainerCache,
            containerEventBroadcaster, logger);

        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }

    private async ValueTask OnImageCreated(DaemonImageEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        if (eventInfo.Image is null) return;

        var imageEntity = eventInfo.Image.Map(platformId);

        var workItem = new PersistPulledImageWorkItem(
            imageEntity,
            platformId,
            imageStream,
            notificationQueue
        );

        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
    }

    private ValueTask OnImageDeleted(DaemonImageEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var item = new ImageDeletedWorkItem(platformId, eventInfo, notificationQueue, dockerDaemonHub, logger);
        return dbWorkQueue.EnqueueAsync(item, cancellationToken);
    }

    private ValueTask OnVolumeEvent(DaemonVolumeEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var notificationItem = new SendVolumeNotificationWorkItem(eventInfo, platformId, dockerDaemonHub);
        return notificationQueue.EnqueueAsync(notificationItem, cancellationToken);
    }

    private ValueTask OnNetworkEvent(DaemonNetworkEventInfo eventInfo, Guid platformId, CancellationToken cancellationToken)
    {
        var notificationItem = new SendNetworkNotificationWorkItem(eventInfo, platformId, dockerDaemonHub);
        return notificationQueue.EnqueueAsync(notificationItem, cancellationToken);
    }
}

internal class SendVolumeNotificationWorkItem(DaemonVolumeEventInfo eventInfo, Guid platformId, IDockerDaemonStreamManager dockerDaemonHub) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => dockerDaemonHub.SendVolumeEvent(eventInfo.Volume, eventInfo?.Action, eventInfo?.VolumeId, platformId);
}

internal class SendNetworkNotificationWorkItem(DaemonNetworkEventInfo eventInfo, Guid platformId, IDockerDaemonStreamManager dockerDaemonHub) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => dockerDaemonHub.SendNetworkEvent(eventInfo.Network, eventInfo.Action, eventInfo.NetworkId, platformId);
}
