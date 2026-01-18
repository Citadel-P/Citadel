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
            var existing = await uow.Containers.GetContainerInfoAsync(
                eventInfo.ContainerId,
                cancellationToken);

            if (existing is null) return;

            if (existing.DeploymentId != null)
            {
                await UpdateDeployment(uow, existing.DeploymentId.Value, cancellationToken);
            }

            await UpdateContainer(uow, existing, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container updated event {ContainerId}", eventInfo.ContainerId);
        }
    }

    private async Task UpdateContainer(IUnitOfWork uow, Container container, CancellationToken cancellationToken)
    {
        container.PartialUpdate(
                state: eventInfo.Container?.State,
                ports: eventInfo.Container?.Ports);

        await uow.Containers.UpdateAsync(container, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var workItem = new ContainerNotificationWorkItem(container, eventInfo, dockerDaemonHub, containerEventBroadcaster);
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
    }

    private async Task UpdateDeployment(IUnitOfWork uow, Guid deploymentId, CancellationToken cancellationToken)
    {
        var existing = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (existing is null) return;
        
        existing.PartialUpdate(
                status: Deployment.ToDeploymentStatus(eventInfo.Container?.State ?? ContainerStateStatus.Unknown));

        await uow.Deployments.UpdateAsync(existing, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, existing);
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
    }
}
