using Application.Services;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class CleanupJob(IServiceScopeFactory scopeFactory, IDelayWithJitterService delayWithJitterService, ILogger<CleanupJob> logger) : BackgroundService
{
    private const int stats_purgeDays = 7;
    private const int activities_purgeDays = 90;
    private const int action_runs_purgeDays = 90;
    private const int checkIntervalInHours = 12;

    protected override Task ExecuteAsync(CancellationToken cancellationToken)
    {
        return delayWithJitterService.DelayWithJitterForAsync(RunPeriodicCleanUp, cancellationToken: cancellationToken);
    }

    private async Task RunPeriodicCleanUp(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                // Remove old stats
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                var thresholdDate = DateTimeOffset.UtcNow.AddDays(-stats_purgeDays);
                var thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                var countStats = await uow.ContainerStats.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);
                countStats += await uow.PlatformStats.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);
                
                // Remove old activities
                thresholdDate = DateTimeOffset.UtcNow.AddDays(-activities_purgeDays);
                thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                var countActivities = await uow.ActivityEventRepository.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);

                // Remove completed automation run history
                thresholdDate = DateTimeOffset.UtcNow.AddDays(-action_runs_purgeDays);
                var countActionRuns = await uow.ActionRuns.RemoveCompletedOlderThanAsync(thresholdDate.UtcDateTime, cancellationToken);

                var countRefreshTokens = await uow.RefreshTokens.DeleteExpiredAsync(DateTime.UtcNow, cancellationToken);

                await uow.CommitAsync(cancellationToken);

                if (countStats > 0)
                    logger.LogInformation("Purged {Count} statistics entries older than {PurgeDays} days.", countStats, stats_purgeDays);

                if (countActivities > 0)
                    logger.LogInformation("Purged {Count} activities entries older than {PurgeDays} days.", countActivities, activities_purgeDays);

                if (countActionRuns > 0)
                    logger.LogInformation("Purged {Count} automation action runs older than {PurgeDays} days.", countActionRuns, action_runs_purgeDays);

                if (countRefreshTokens > 0)
                    logger.LogInformation("Purged {Count} expired refresh tokens.", countRefreshTokens);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while running cleanup job.");
            }
            finally
            {
                await Task.Delay(TimeSpan.FromHours(checkIntervalInHours), cancellationToken);
            }
        }
    }
}
