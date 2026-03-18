using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Alerts;

namespace Application.Services.SignalR;

public interface IAlertEventStreamManager : IStreamGroupManager
{
    Task SendTriggeredAlertEvent(AlertEvent alertEvent);
    Task SendUpdatedAlertEvents(IEnumerable<AlertEvent> alertEvents);
    Task SendUnresolvedAlertCount(int count);
}

internal sealed class AlertEventStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IAlertEventStreamManager
{
    public Task SendTriggeredAlertEvent(AlertEvent alertEvent)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendTriggeredAlertEvent(alertEvent);
    }

    public Task SendUpdatedAlertEvents(IEnumerable<AlertEvent> alertEvents)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendUpdatedAlertEvents(alertEvents);
    }

    public Task SendUnresolvedAlertCount(int count)
    {
        if (streams.IsEmpty)
            return Task.CompletedTask;

        return dispatcher.SendUnresolvedAlertCount(count);
    }
}
