using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;

namespace Application.Features.Alerters.Notifications;

internal sealed class TriggeredAlertEventNotificationWorkItem(
    AlertEvent alertEvent,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => alertEventStreamManager.SendTriggeredAlertEvent(alertEvent);
}

internal sealed class UpdatedAlertEventsNotificationWorkItem(
    IEnumerable<AlertEvent> alertEvents,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => alertEventStreamManager.SendUpdatedAlertEvents(alertEvents);
}

internal sealed class UnresolvedAlertCountNotificationWorkItem(
    int count,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => alertEventStreamManager.SendUnresolvedAlertCount(count);
}
