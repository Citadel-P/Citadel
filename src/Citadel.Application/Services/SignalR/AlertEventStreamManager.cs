using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Alerts;

namespace Application.Services.SignalR;

public interface IAlertEventStreamManager : IStreamGroupManager
{
    Task SendTriggeredAlertEvent(AlertEvent alertEvent, IEnumerable<Guid> userIds);
    Task SendUpdatedAlertEvents(IReadOnlyDictionary<Guid, IReadOnlyCollection<AlertEvent>> alertEventsByUser);
    Task SendUnresolvedAlertCounts(IReadOnlyDictionary<Guid, int> countsByUser);
}

internal sealed class AlertEventStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IAlertEventStreamManager
{
    public Task SendTriggeredAlertEvent(AlertEvent alertEvent, IEnumerable<Guid> userIds)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendTriggeredAlertEvent(alertEvent, userIds);
    }

    public Task SendUpdatedAlertEvents(IReadOnlyDictionary<Guid, IReadOnlyCollection<AlertEvent>> alertEventsByUser)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendUpdatedAlertEvents(alertEventsByUser);
    }

    public Task SendUnresolvedAlertCounts(IReadOnlyDictionary<Guid, int> countsByUser)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendUnresolvedAlertCounts(countsByUser);
    }
}
