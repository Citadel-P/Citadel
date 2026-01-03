using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs.WorkItems;

internal sealed class ContainerUpdatedWorkItem(
    DaemonContainerEventInfo eventInfo,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    ILogger logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        try
        {
            var existing = await uow.Containers.GetByIdAsync(
                eventInfo.ContainerId,
                cancellationToken);

            if (existing is null) return;

            existing.PartialUpdate(
                state: eventInfo.Container?.State,
                ports: eventInfo.Container?.Ports);

            await uow.Containers.UpdateAsync(existing, cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var workItem = new SendContainerNotificationWorkItem(dockerDaemonHub, existing, eventInfo.Action);
            await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to handle container updated event {ContainerId}", eventInfo.ContainerId);
        }
    }
}

internal class SendContainerNotificationWorkItem(IDockerDaemonStreamManager dockerDaemonHub, Container container, string action) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => dockerDaemonHub.SendContainerEvent(container, action);
}