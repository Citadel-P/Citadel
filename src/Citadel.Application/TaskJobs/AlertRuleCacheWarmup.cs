using Application.Services.Alerts;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class AlertRuleCacheWarmup(IServiceScopeFactory scopeFactory, AlertRuleCache alertRuleCache,
    ILogger<CleanupJob> logger) : BackgroundService
{
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(12);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await alertRuleCache.ReloadAsync(stoppingToken);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during periodic {JobName} synchronization.", nameof(AlertRuleCacheWarmup));
            }

            await Task.Delay(SyncInterval, stoppingToken);
        }
    }
}
