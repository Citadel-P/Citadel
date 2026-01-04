using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;

internal sealed class ContainerUpdatedWorkItem(
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
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

            existing.PartialUpdate(
                state: eventInfo.Container?.State,
                ports: eventInfo.Container?.Ports);

            await uow.Containers.UpdateAsync(existing, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var workItem = new ContainerNotificationWorkItem(existing, eventInfo, dockerDaemonHub, containerEventBroadcaster);
            await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container updated event {ContainerId}", eventInfo.ContainerId);
        }
    }
}
