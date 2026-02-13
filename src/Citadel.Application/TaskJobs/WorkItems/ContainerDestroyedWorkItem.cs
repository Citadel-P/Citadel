using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;


internal sealed class ContainerDestroyedWorkItem(
    Guid platformId,
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IDeploymentStreamManager deploymentHub,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            ActivityEvent? activityEvent = null;
            Deployment? deployment = null;
            Image? image = null;
            var existing = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
            if (existing is null) return;

            platformContainerCache.TryRemoveContainer(platformId, existing.DockerContainerId);

            if (!string.IsNullOrEmpty(existing.DockerImageId))
            {
                image = await UpdateImage(uow, existing, cancellationToken);
            }

            if (existing.DeploymentId != null)
            {
                (deployment, activityEvent) = await UpdateDeploymentStatus(uow, existing.DeploymentId.Value, DeploymentStatus.Degraded, existing.DockerContainerId, cancellationToken);
            }

            await uow.Containers.DeleteAsync([existing.Id], cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var containerNotification = new ContainerNotificationWorkItem(
                existing,
                eventInfo,
                dockerDaemonHub,
                containerEventBroadcaster);

            await notificationQueue.EnqueueAsync(containerNotification, cancellationToken);
            if (deployment != null)
            {
                var deploymentNotification = new DeploymentNotificationWorkItem(deploymentHub, deployment);
                await notificationQueue.EnqueueAsync(deploymentNotification, cancellationToken);
            }
            if (image != null)
            {
                var imageNotificationItem = new ImageNotificationWorkItem(dockerDaemonHub, image, "update");
                await notificationQueue.EnqueueAsync(imageNotificationItem, cancellationToken);
            }
            if (activityEvent != null)
            {
                await notificationQueue.EnqueueAsync(new ActivityNotification(activityEvent), cancellationToken);
            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container destroyed event {ContainerId}", eventInfo.ContainerId);
        }
    }

    private async Task<Image?> UpdateImage(IUnitOfWork uow, Container container, CancellationToken cancellationToken)
    {
        var image = await uow.Images.GetByDockerImageIdAsync(container.DockerImageId!, platformId, cancellationToken);
        if (image is null) return null;

        image.PartialUpdate(containers: image.Containers - 1);
        await uow.Images.AddOrUpdateAsync(image, cancellationToken);

        return image;
    }

    internal static async Task<(Deployment?, ActivityEvent?)> UpdateDeploymentStatus(IUnitOfWork uow, Guid deploymentId, 
        DeploymentStatus status, string containerId, CancellationToken cancellationToken)
    {
        var deployment = await uow.Deployments.GetInfoAsync(deploymentId, cancellationToken);
        if (deployment is null) return (null, null);

        EventInfo? eventInfo = status switch
        {
            DeploymentStatus.Healthy => new DeploymentStarted([containerId]),
            DeploymentStatus.Stopped => new DeploymentStopped([containerId]),
            DeploymentStatus.Pending => new DeploymentPaused([containerId]),
            DeploymentStatus.Degraded => new DeploymentDegraded($"The associated container was deleted (ID: {containerId})."),
            _ => null
        };

        ActivityEventType type = status switch
        {
            DeploymentStatus.Healthy => ActivityEventType.DeploymentStarted,
            DeploymentStatus.Stopped => ActivityEventType.DeploymentStopped,
            DeploymentStatus.Pending => ActivityEventType.DeploymentPaused,
            DeploymentStatus.Degraded => ActivityEventType.DeploymentDegraded,
            _ => ActivityEventType.DeploymentDegraded
        };

        if (eventInfo == null)
        {
            return (deployment, null);
        }

        // When a deployment is applying, we don't want to update the status & log the activities
        ActivityEvent? activity = null;
        var deploymentInProgress = deployment.Status == DeploymentStatus.Applying;
        if (!deploymentInProgress)
        {
            activity = new ActivityEvent(
                    info: eventInfo,
                    eventType: type,
                    resourceId: deployment.Id,
                    platformId: deployment.PlatformId,
                    resourceName: deployment.Name,
                    status: eventInfo is DeploymentDegraded ? ActivityStatus.Warning : ActivityStatus.Success,
                    actorId: deployment.ControlTriggeredBy ?? Constants.SystemId
                    );

            await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }
        

        deployment.ReleaseProcessing(status);
        await uow.Deployments.UpdateProcessingAsync(
            id: deployment.Id,
            status: deploymentInProgress ? DeploymentStatus.Applying : deployment.Status,
            state: deployment.ControlState,
            startedAt: deployment.ControlStartedAt,
            rowVersion: deployment.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: deployment.ControlTriggeredBy,
            cancellationToken);

        return (deployment, activity);
    }
}