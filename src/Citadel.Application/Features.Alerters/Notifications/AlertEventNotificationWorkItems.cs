using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;

namespace Application.Features.Alerters.Notifications;

internal sealed class TriggeredAlertEventNotificationWorkItem(
    AlertEvent alertEvent,
    IEnumerable<Guid> userIds,
    IReadOnlyDictionary<Guid, int> unresolvedCountsByUser,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => Task.WhenAll(
            alertEventStreamManager.SendTriggeredAlertEvent(alertEvent, userIds),
            alertEventStreamManager.SendUnresolvedAlertCounts(unresolvedCountsByUser));
}

internal sealed class UpdatedAlertEventsNotificationWorkItem(
    IReadOnlyDictionary<Guid, IReadOnlyCollection<AlertEvent>> alertEventsByUser,
    IReadOnlyDictionary<Guid, int> unresolvedCountsByUser,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => Task.WhenAll(
            alertEventStreamManager.SendUpdatedAlertEvents(alertEventsByUser),
            alertEventStreamManager.SendUnresolvedAlertCounts(unresolvedCountsByUser));
}

internal sealed class UnresolvedAlertCountNotificationWorkItem(
    IReadOnlyDictionary<Guid, int> countsByUser,
    IAlertEventStreamManager alertEventStreamManager) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => alertEventStreamManager.SendUnresolvedAlertCounts(countsByUser);
}
