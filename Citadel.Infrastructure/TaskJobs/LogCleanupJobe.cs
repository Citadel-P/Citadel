using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Infrastructure.TaskJobs
{
    internal class LogCleanupJob(ILogger<ContainersInfoJob> logger) : BackgroundService
    {
        private const int RetentionDays = 10;

        protected override async Task ExecuteAsync(CancellationToken cancellationToken)
        {
            while (!cancellationToken.IsCancellationRequested)
            {
                try
                {
                    await RunJob(cancellationToken);
                }
                catch (Exception ex)
                {
                    logger.LogError(ex, "Unhandled exception in {Message}", ex.Message);
                }

                await Task.Delay(TimeSpan.FromHours(24), cancellationToken);
            }
        }

        public Task RunJob(CancellationToken cancellationToken)
        {
            try
            {
                var logFiles = Directory.GetFiles(Constants.LogPath, "*.log");
                var cutoffDate = DateTime.UtcNow.AddDays(-RetentionDays);

                foreach (var file in logFiles)
                {
                    var fileInfo = new FileInfo(file);
                    if (fileInfo.LastWriteTimeUtc < cutoffDate)
                    {
                        fileInfo.Delete();
                    }
                }

                logger.LogInformation("Log cleanup completed. Deleted files older than {RetentionDays} days.", RetentionDays);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Log cleanup failed: {Message}", ex.Message);
            }

            return Task.CompletedTask;
        }
    }
}
