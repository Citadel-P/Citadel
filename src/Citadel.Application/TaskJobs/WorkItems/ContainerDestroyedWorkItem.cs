using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;


internal sealed class ContainerDestroyedWorkItem(
    Guid platformId,
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
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

            await uow.Containers.DeleteAsync([existing.Id], cancellationToken);

            // update image containers count if any
            if (!string.IsNullOrEmpty(existing.DockerImageId))
            {
                var image = await uow.Images.GetByImageIdAsync(existing.DockerImageId, platformId, cancellationToken);
                if (image != null)
                {
                    image.PartialUpdate(containers: image.Containers - 1);
                    await uow.Images.AddOrUpdateAsync(image, cancellationToken);

                    await uow.CommitAsync(cancellationToken);

                    var imageNotificationItem = new SendImageNotificationWorkItem(image, "update", dockerDaemonHub);
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

            var notificationItem = new ContainerCreatedNotificationWorkItem(
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
}