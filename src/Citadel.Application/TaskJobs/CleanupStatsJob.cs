using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class CleanupStatsJob(IServiceScopeFactory scopeFactory, ILogger<CleanupStatsJob> logger) : BackgroundService
{
    private const int purgeDays = 2;
    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        // Run cleanup every 3 hours
        while (!cancellationToken.IsCancellationRequested)
        {
            await Task.Delay(TimeSpan.FromHours(3), cancellationToken);
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                var thresholdDate = DateTimeOffset.UtcNow.AddDays(-purgeDays);
                var thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                await uow.ContainerStats.RemoveOlderThanAsync(thresholdEpochSeconds, cancellationToken);
                await uow.CommitAsync();

                logger.LogInformation("Removed container statistics entries older than {PurgeDays} days.", purgeDays);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while removing container statistics.");
            }
        }
    }
}