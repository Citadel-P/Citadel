using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;

internal sealed class ContainerUpdatedWorkItem(
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IDeploymentStreamManager deploymentHub,
    IDockerDaemonStreamManager dockerDaemonHub,
    IContainerEventBroadcaster containerEventBroadcaster,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            ActivityEvent? activityEvent = null;
            Deployment? deployment = null;
            Container? container = null;
            var existing = await uow.Containers.GetContainerInfoAsync(
                eventInfo.ContainerId,
                cancellationToken);

            if (existing is null) return;

            if (existing.DeploymentId != null)
            {
                var status = Deployment.ToDeploymentStatus(eventInfo.Container?.State ?? ContainerStateStatus.Unknown);
                (deployment, activityEvent) = await ContainerDestroyedWorkItem.UpdateDeploymentStatus(uow, existing.DeploymentId.Value, status, cancellationToken);
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
                state : eventInfo.Container?.State,
                ports: eventInfo.Container?.Ports);
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
