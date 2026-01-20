using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
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
            var existing = await uow.Containers.GetByIdAsync(eventInfo.ContainerId, cancellationToken);
            if (existing is null) return;

            platformContainerCache.TryRemoveContainer(platformId, existing.DockerContainerId);

            if (!string.IsNullOrEmpty(existing.DockerImageId))
            {
                await UpdateImage(uow, existing, cancellationToken);
            }

            if (existing.DeploymentId != null)
            {
                await UpdateDeployment(uow, existing.DeploymentId.Value, cancellationToken);
            }

            await uow.Containers.DeleteAsync([existing.Id], cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var notificationItem = new ContainerNotificationWorkItem(
                existing,
                eventInfo,
                dockerDaemonHub,
                containerEventBroadcaster);

            await notificationQueue.EnqueueAsync(notificationItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container destroyed event {ContainerId}", eventInfo.ContainerId);
        }
    }

    private async Task UpdateImage(IUnitOfWork uow, Domain.Entities.Container container, CancellationToken cancellationToken)
    {
        var image = await uow.Images.GetByDockerImageIdAsync(container.DockerImageId!, platformId, cancellationToken);
        if (image != null)
        {
            // will commit in the main method
            image.PartialUpdate(containers: image.Containers - 1);
            await uow.Images.AddOrUpdateAsync(image, cancellationToken);

            var imageNotificationItem = new SendImageNotificationWorkItem(dockerDaemonHub, image, "update");
            await notificationQueue.EnqueueAsync(imageNotificationItem, cancellationToken);
        }
    }

    private async Task UpdateDeployment(IUnitOfWork uow, Guid deploymentId, CancellationToken cancellationToken)
    {
        var existing = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (existing is null) return;

        // will commit in the main method
        existing.PartialUpdate(status: DeploymentStatus.Degraded);
        await uow.Deployments.UpdateAsync(existing, cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, existing);
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
    }
}