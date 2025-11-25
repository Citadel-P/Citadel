using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.Repositories.DbQueue;

internal sealed class NotificationWorker(INotificationQueue queue, ILogger<NotificationWorker> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var reader = queue.Reader;

        await foreach (var item in reader.ReadAllAsync(stoppingToken))
        {
            try
            {
                await item.ExecuteAsync(stoppingToken);
            }
            catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
            {
                // Normal shutdown
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while executing notification work item..");
            }
        }
    }
}
