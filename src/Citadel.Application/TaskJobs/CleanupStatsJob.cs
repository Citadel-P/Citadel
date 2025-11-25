using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class CleanupStatsJob(IServiceScopeFactory scopeFactory, ILogger<CleanupStatsJob> logger) : BackgroundService
{
    private const int purgeDays = 3;
    private const int checkIntervalInHours = 12;

    protected override Task ExecuteAsync(CancellationToken cancellationToken)
    {
        return Helpers.DelayWithJitterFor(RunPeriodicPurge, cancellationToken: cancellationToken);
    }

    private async Task RunPeriodicPurge(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                var thresholdDate = DateTimeOffset.UtcNow.AddDays(-purgeDays);
                var thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                await uow.ContainerStats.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);
                await uow.PlatformStats.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);
                await uow.CommitAsync(cancellationToken);

                logger.LogInformation("Removed statistics entries older than {PurgeDays} days.", purgeDays);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while removing statistics.");
            }
            finally
            {
                await Task.Delay(TimeSpan.FromHours(checkIntervalInHours), cancellationToken);
            }
        }
    }
}