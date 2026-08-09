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

            if (!string.IsNullOrEmpty(existing.DockerImageId))
            {
                image = await UpdateImage(uow, existing, cancellationToken);
            }

            if (existing.DeploymentId != null)
            {
                (deployment, activityEvent) = await UpdateDeploymentStatus(uow, existing.DeploymentId.Value, DeploymentStatus.Degraded, existing.DockerContainerId, cancellationToken);
            }

            if (existing.StackId != null && !existing.IsSwarmTask)
            {
                (stack, activityEvent) = await UpdateStackStatus(
                    uow,
                    existing.StackId.Value,
                    [new StackContainerState(existing.DockerContainerId, eventInfo.Container?.State ?? ContainerStateStatus.Unknown)],
                    eventInfo.Container?.State ?? ContainerStateStatus.Unknown,
                    existing.DockerContainerId,
                    StackReleaseStatus.Degraded,
                    cancellationToken,
                    allowDegradedWhileProcessing: existing.ControlState == ResourceControlState.Processing);
            }

            await uow.Containers.DeleteAsync([existing.Id], cancellationToken);
            await uow.CommitAsync(cancellationToken);

            // Remove the cache entry only after the durable delete succeeds.
            platformContainerCache.TryRemoveContainer(platformId, existing.DockerContainerId);

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

            if (stack != null && await uow.Stacks.ExistsAsync(stack.Id, cancellationToken))
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
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
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
        Guid stackId,
        IEnumerable<StackContainerState> containers,
        ContainerStateStatus changedState,
        string changedContainerId,
        StackReleaseStatus? forcedStatus,
        CancellationToken cancellationToken,
        bool allowDegradedWhileProcessing = false)
    {
        var stack = await uow.Stacks.GetInfoAsync(stackId, cancellationToken);
        if (stack is null) return (null, null);

        var containerStates = containers.ToArray();
        var status = forcedStatus ?? Stack.ToStackStatus(containerStates.Select(x => x.State));

        if (ShouldSkipStackStatusChange(stack, status, allowDegradedWhileProcessing))
        {
            return (null, null);
        }

        ActivityEvent? activity = null;
        var activityDescriptor = CreateStackActivity(status, containerStates, changedState, changedContainerId);
        if (activityDescriptor is not null)
        {
            activity = new ActivityEvent(
                    info: activityDescriptor.Info,
                    eventType: activityDescriptor.EventType,
                    resourceId: stack.Id,
                    platformId: stack.CurrentStackRelease?.PlatformId,
                    resourceName: stack.Name,
                    status: activityDescriptor.Status,
                    actorId: stack.ControlTriggeredBy ?? Constants.SystemId
                    );

            await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        }

        stack.ReleaseProcessing(status);
        await uow.Stacks.UpdateProcessingAsync(
            id: stack.Id,
            status: stack.CurrentStackRelease?.Status ?? StackReleaseStatus.Unknown,
            state: stack.ControlState,
            startedAt: stack.ControlStartedAt,
            rowVersion: stack.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: stack.ControlTriggeredBy,
            cancellationToken);

        return (stack, activity);
    }

    private static bool ShouldSkipStackStatusChange(
        Stack stack,
        StackReleaseStatus status,
        bool allowDegradedWhileProcessing)
    {
        var currentStatus = stack.CurrentStackRelease?.Status;
        if (currentStatus == status) return true;
        if (currentStatus == StackReleaseStatus.Applying) return true;

        if (allowDegradedWhileProcessing && status == StackReleaseStatus.Degraded)
        {
            return false;
        }

        return stack.ControlState == ResourceControlState.Processing
            && status is StackReleaseStatus.Pending or StackReleaseStatus.Degraded;
    }

    private static StackActivityDescriptor? CreateStackActivity(
        StackReleaseStatus status,
        IEnumerable<StackContainerState> containers,
        ContainerStateStatus changedState,
        string changedContainerId)
    {
        return status switch
        {
            StackReleaseStatus.Healthy => new StackActivityDescriptor(
                new StackStarted(GetContainerIds(containers, ContainerStateStatus.Running)),
                ActivityEventType.StackStarted,
                ActivityStatus.Success),
            StackReleaseStatus.Stopped => new StackActivityDescriptor(
                new StackStopped(GetContainerIds(containers, ContainerStateStatus.Exited, ContainerStateStatus.Offline)),
                ActivityEventType.StackStopped,
                ActivityStatus.Success),
            StackReleaseStatus.Paused => new StackActivityDescriptor(
                new StackPaused(GetContainerIds(containers, ContainerStateStatus.Paused)),
                ActivityEventType.StackPaused,
                ActivityStatus.Success),
            StackReleaseStatus.Degraded => new StackActivityDescriptor(
                new StackDegraded(GetDegradedReason(changedState, changedContainerId)),
                ActivityEventType.StackDegraded,
                ActivityStatus.Warning),
            _ => null
        };
    }

    private static string GetDegradedReason(ContainerStateStatus changedState, string changedContainerId)
    {
        return changedState != ContainerStateStatus.Running
            ? $"Container {changedContainerId} changed state to {changedState}, causing the stack to become degraded."
            : "One or more associated containers are not running normally.";
    }

    private static string[] GetContainerIds(IEnumerable<StackContainerState> containers, params ContainerStateStatus[] states)
    {
        var acceptedStates = states.ToHashSet();
        return [.. containers
            .Where(container => acceptedStates.Contains(container.State))
            .Select(container => container.ContainerId)
            .Distinct(StringComparer.OrdinalIgnoreCase)];
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

internal sealed record StackContainerState(string ContainerId, ContainerStateStatus State);

internal sealed record StackActivityDescriptor(
    ActivityEventInfo Info,
    ActivityEventType EventType,
    ActivityStatus Status);
