using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;

internal sealed class ContainerUpdatedWorkItem(
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IDeploymentStreamManager deploymentHub,
    IDockerDaemonStreamManager dockerDaemonHub,
    IContainerEventBroadcaster containerEventBroadcaster,
    IStackStreamManager stackHub,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            ActivityEvent? activityEvent = null;
            Deployment? deployment = null;
            Container? container = null;
            Stack? stack = null;

            var existing = await uow.Containers.GetContainerInfoAsync(eventInfo.ContainerId, cancellationToken);

            if (existing is null) return;

            if (existing.DeploymentId != null)
            {
                var status = Deployment.ToDeploymentStatus(eventInfo.Container?.State ?? ContainerStateStatus.Unknown);
                (deployment, activityEvent) = await ContainerDestroyedWorkItem.UpdateDeploymentStatus(uow, existing.DeploymentId.Value, status, existing.DockerContainerId, cancellationToken);
            }

            if (existing.StackId != null && !existing.IsSwarmTask)
            {
                var containers = await uow.Stacks.GetContainersAsync(existing.StackId.Value, cancellationToken);
                var changedState = eventInfo.Container?.State ?? ContainerStateStatus.Unknown;
                var containerStates = containers.Select(container =>
                    new StackContainerState(
                        container.DockerContainerId,
                        string.Equals(container.DockerContainerId, existing.DockerContainerId, StringComparison.OrdinalIgnoreCase)
                            ? changedState
                            : container.State));

                (stack, activityEvent) = await ContainerDestroyedWorkItem.UpdateStackStatus(
                    uow,
                    existing.StackId.Value,
                    containerStates,
                    changedState,
                    existing.DockerContainerId,
                    forcedStatus: null,
                    cancellationToken: cancellationToken,
                    allowDegradedWhileProcessing: existing.ControlState == ResourceControlState.Processing);
            }

            container = await UpdateContainer(uow, existing, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            // Notify Container
            var workItem = new ContainerNotificationWorkItem(container, eventInfo, dockerDaemonHub, containerEventBroadcaster);
            await notificationQueue.EnqueueAsync(workItem, cancellationToken);

            // Notify Deployment
            if (deployment != null)
            {
                var deploymentWorkItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
                await notificationQueue.EnqueueAsync(deploymentWorkItem, cancellationToken);
            }

            // Notify Stack
            if (stack != null && await uow.Stacks.ExistsAsync(stack.Id, cancellationToken))
            {
                var stackWorkItem = new StackNotificationWorkItem(stackHub, stack);
                await notificationQueue.EnqueueAsync(stackWorkItem, cancellationToken);
            }

            // Notify Activity
            if (activityEvent != null)
            {
                var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activityEvent.AssignActor(uow, cancellationToken));
                await notificationQueue.EnqueueAsync(activityWorkItem, cancellationToken);
            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container updated event {ContainerId}", eventInfo.ContainerId);
        }
    }

    private async Task<Container> UpdateContainer(IUnitOfWork uow, Container container, CancellationToken cancellationToken)
    {
        container.PartialUpdate(
            name: eventInfo.Container?.Name,
            state: eventInfo.Container?.State,
            ports: eventInfo.Container?.Ports,
            isSystem: eventInfo.Container?.IsSystem,
            systemRole: eventInfo.Container?.SystemRole,
            hasCitadelOwnershipLabels: eventInfo.Container?.HasCitadelOwnershipLabels);
        await uow.Containers.UpdateAsync(container, cancellationToken);

        container.ReleaseProcessing();

        await uow.Containers.UpdateProcessingAsync(
            id: container.Id,
            state: container.ControlState,
            startedAt: container.ControlStartedAt,
            rowVersion: container.RowVersion,
            checkRowVersion: false,
            controlTriggeredBy: container.ControlTriggeredBy,
            cancellationToken);

        return container;
    }
}
