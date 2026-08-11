using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs.WorkItems;

internal sealed class ContainerCreatedWorkItem(
    DaemonContainerEventInfo eventInfo,
    Guid platformId,
    INotificationQueue notificationQueue,
    ChannelWriter<UnmanagedContainerAlertRequest> unmanagedContainerAlertWriter,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        if (eventInfo.Container is null) return;

        var dockerImageId = string.IsNullOrWhiteSpace(eventInfo.Container.ImageId)
            ? eventInfo.Container.Image
            : eventInfo.Container.ImageId;
        if (string.IsNullOrWhiteSpace(dockerImageId))
        {
            logger.LogWarning(
                "Ignoring container created event without an image identity for platform {PlatformId}",
                platformId);
            return;
        }

        var dockerContainer = eventInfo.Container with { ImageId = dockerImageId };

        try
        {
            var platform = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
            var managerNodeId = (platform?.PlatformDescriptor as DockerSwarmPlatformDescriptor)?.NodeID;
            var image = await uow.Images.GetByDockerImageIdAsync(
                dockerImageId,
                platformId,
                cancellationToken);

            var observedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var container = dockerContainer.Map(
                platformId,
                image?.Id,
                managerNodeId,
                managerNodeId is null ? null : observedAt);

            await uow.Containers.AddAsync(container, cancellationToken);

            // update image status
            if (!string.IsNullOrEmpty(container.DockerImageId))
            {
                var imageToUpdate = await uow.Images.GetByDockerImageIdAsync(
                    container.DockerImageId,
                    platformId,
                    cancellationToken);
                if (imageToUpdate != null)
                {
                    imageToUpdate.PartialUpdate(containers: imageToUpdate.Containers + 1);
                    await uow.Images.AddOrUpdateAsync(imageToUpdate, cancellationToken);

                    // commit for both container + image
                    await uow.CommitAsync(cancellationToken);

                    var imageNotificationItem = new ImageNotificationWorkItem(dockerDaemonHub, imageToUpdate, "update");
                    await notificationQueue.EnqueueAsync(imageNotificationItem, cancellationToken);
                }
                else
                {
                    await uow.CommitAsync(cancellationToken);
                }
            }
            else
            {
                await uow.CommitAsync(cancellationToken);
            }

            // Publish the cache entry only after the durable container write succeeds.
            platformContainerCache.TryAddContainer(platformId, container.DockerContainerId, container.Id);

            var notificationItem = new ContainerNotificationWorkItem(
                container,
                eventInfo,
                dockerDaemonHub, 
                containerEventBroadcaster);

            await notificationQueue.EnqueueAsync(notificationItem, cancellationToken);

            if (!container.IsSystem && !container.HasCitadelOwnershipLabels && !dockerContainer.IsSwarmTask)
            {
                await unmanagedContainerAlertWriter.WriteAsync(
                        new UnmanagedContainerAlertRequest(platformId, container.DockerContainerId),
                        cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container created event for platform {PlatformId}", platformId);
        }
    }
}

internal class ContainerNotificationWorkItem(
    Container container,
    DaemonContainerEventInfo eventInfo,
    IDockerDaemonStreamManager dockerDaemonHub, 
    IContainerEventBroadcaster containerEventBroadcaster) : INotificationWorkItem
{
    public async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        await dockerDaemonHub.SendContainerEvent(container, eventInfo.Action);
        await containerEventBroadcaster.PublishAsync(
            new ContainerEvent(
                container.PlatformId,
                container.DockerNodeId,
                container.DockerContainerId,
                eventInfo.Action),
            cancellationToken);
    }
}

internal class ImageNotificationWorkItem(IDockerDaemonStreamManager dockerDaemonHub, Image image, string action) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => dockerDaemonHub.SendImageEvent(image, action);
}

internal class DeploymentNotificationWorkItem(IDeploymentStreamManager deploymentHub, Deployment deployment, string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => deploymentHub.SendDeploymentInfo(deployment, action);
}
