using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;

internal sealed class ImageDeletedWorkItem(
    Guid platformId,
    DaemonImageEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var existing = await uow.Images.GetByDockerImageIdAsync(eventInfo.ImageId, platformId, cancellationToken);
            if (existing is null) return;

            await uow.Images.DeleteAsync([existing.Id], cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var imageNotificationItem = new SendImageNotificationWorkItem(existing, eventInfo.Action, dockerDaemonHub);
            await notificationQueue.EnqueueAsync(imageNotificationItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle image deleted event {ImageId}", eventInfo.ImageId);
        }
    }
}
