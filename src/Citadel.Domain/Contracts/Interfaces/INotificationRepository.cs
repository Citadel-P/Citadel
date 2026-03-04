using Domain.Entities.Alerts;

namespace Domain.Contracts.Interfaces;

public interface INotificationRepository
{
    Task SendAlertAsync(AlertEvent alertEvent, IEnumerable<AlertChannel> channels, CancellationToken cancellationToken);
    Task<NotificationResult> SendTestNotificationAsync(AlertChannel channel, CancellationToken cancellationToken);
}

public record NotificationResult(bool IsSuccess, string? ErrorMessage = null);