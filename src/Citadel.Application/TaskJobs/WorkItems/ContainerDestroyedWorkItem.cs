using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;


internal sealed class ContainerDestroyedWorkItem(
    Guid platformId,
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IStackStreamManager stackHub,
    IActivityStreamManager activityHub,
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
            Stack? stack = null;
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

            if (existing.StackId != null)
            {
                (stack, activityEvent) = await UpdateStackStatus(uow, existing.StackId.Value, StackReleaseStatus.Degraded, eventInfo.Container?.State ?? ContainerStateStatus.Unknown, existing.DockerContainerId, cancellationToken);
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

            if (stack != null)
            {
                var stackNotification = new StackNotificationWorkItem(stackHub, stack);
                await notificationQueue.EnqueueAsync(stackNotification, cancellationToken);
            }

            if (image != null)
            {
                var imageNotificationItem = new ImageNotificationWorkItem(dockerDaemonHub, image, "update");
                await notificationQueue.EnqueueAsync(imageNotificationItem, cancellationToken);
            }

            if (activityEvent != null)
            {
                await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activityEvent.AssignActor(uow, cancellationToken)), cancellationToken);
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

    internal static async Task<(Stack?, ActivityEvent?)> UpdateStackStatus(IUnitOfWork uow,
        Guid stackId, StackReleaseStatus status, ContainerStateStatus state, string containerId, CancellationToken cancellationToken)
    {
        var stack = await uow.Stacks.GetInfoAsync(stackId, cancellationToken);
        if (stack is null) return (null, null);
        // Docker daemon may emit multiple container events for a stack, resulting in duplicate activity entries.
        // Todo: Aggregate events and only create one activity event per stack per status change.
        var previousState = stack.CurrentStackRelease?.Status;
        if (previousState == status) return (stack, null);

        ActivityEventInfo? eventInfo = status switch
        {
            StackReleaseStatus.Healthy => new StackStarted([containerId]),
            StackReleaseStatus.Stopped => new StackStopped([containerId]),
            StackReleaseStatus.Pending => new StackPaused([containerId]),
            StackReleaseStatus.Degraded => new StackDegraded(state != ContainerStateStatus.Running 
                ? $"Container {containerId} changed state to {state}, causing the stack to become degraded."
                : "One or more associated containers are not running normally."),
            _ => null
        };
        
        ActivityEventType type = status switch
        {
            StackReleaseStatus.Healthy => ActivityEventType.StackStarted,
            StackReleaseStatus.Stopped => ActivityEventType.StackStopped,
            StackReleaseStatus.Pending => ActivityEventType.StackPaused,
            StackReleaseStatus.Degraded => ActivityEventType.StackDegraded,
            _ => ActivityEventType.StackDegraded
        };

        if (eventInfo == null)
        {
            return (stack, null);
        }

        // When a stack is applying, we don't want to update the status & log the activities
        ActivityEvent? activity = null;
        var inProgress = stack.CurrentStackRelease?.Status == StackReleaseStatus.Applying;
        if (!inProgress)
        {
            activity = new ActivityEvent(
                    info: eventInfo,
                    eventType: type,
                    resourceId: stack.Id,
                    platformId: stack.CurrentStackRelease?.PlatformId,
                    resourceName: stack.Name,
                    status: eventInfo is StackDegraded ? ActivityStatus.Warning : ActivityStatus.Success,
                    actorId: stack.ControlTriggeredBy ?? Constants.SystemId
                    );

            await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }

        stack.ReleaseProcessing(status);
        await uow.Stacks.UpdateProcessingAsync(
            id: stack.Id,
            status: inProgress ? StackReleaseStatus.Applying : (stack.CurrentStackRelease?.Status ?? StackReleaseStatus.Unknown),
            state: stack.ControlState,
            startedAt: stack.ControlStartedAt,
            rowVersion: stack.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: stack.ControlTriggeredBy,
            cancellationToken);

        return (stack, activity);
    }

    internal static async Task<(Deployment?, ActivityEvent?)> UpdateDeploymentStatus(IUnitOfWork uow, Guid deploymentId,
        DeploymentStatus status, string containerId, CancellationToken cancellationToken)
    {
        var deployment = await uow.Deployments.GetInfoAsync(deploymentId, cancellationToken);
        if (deployment is null) return (null, null);

        ActivityEventInfo? eventInfo = status switch
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