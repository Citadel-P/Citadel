using Domain.Entities.Alerts;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IShoutrrrCliRepository
{
    Task SendAlertAsync(AlertEvent alertEvent, IEnumerable<AlertChannel> channels, string name, CancellationToken cancellationToken);
    Task<Result> SendTestNotificationAsync(AlertChannel channel, CancellationToken cancellationToken);
}