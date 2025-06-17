using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class CleanupStatsJob(IServiceScopeFactory scopeFactory, ILogger<CleanupStatsJob> logger) : BackgroundService
{
    private const int purgeDays = 2;
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        // Run cleanup every 3 hours
        while (!stoppingToken.IsCancellationRequested)
        {
            await Task.Delay(TimeSpan.FromHours(3), stoppingToken);
            try
            {
                using var scope = scopeFactory.CreateScope();
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                var thresholdDate = DateTimeOffset.UtcNow.AddDays(-purgeDays);
                var thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                var oldStats = uow.ContainerStats.Query().Where(stat => stat.Created < thresholdEpochSeconds);

                uow.ContainerStats.RemoveRange(oldStats);
                await uow.SaveChangesAsync(stoppingToken);

                logger.LogInformation($"Cleaned up old container stats older than {purgeDays} days.");
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while cleaning up old container stats.");
            }
        }
    }
}