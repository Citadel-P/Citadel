using Infrastructure.EntityFramework;
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
                using var dbContext = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();

                var thresholdDate = DateTimeOffset.UtcNow.AddDays(-purgeDays);
                var thresholdEpochSeconds = thresholdDate.ToUnixTimeSeconds();

                var oldStats = dbContext.ContainerStats.Where(stat => stat.Created < thresholdEpochSeconds);

                dbContext.ContainerStats.RemoveRange(oldStats);
                await dbContext.SaveChangesAsync(stoppingToken);

                logger.LogInformation($"Cleaned up old container stats older than {purgeDays} days.");
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while cleaning up old container stats.");
            }
        }
    }
}