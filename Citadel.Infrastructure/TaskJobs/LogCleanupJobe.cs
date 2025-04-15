using Hosting.Common;
using Quartz;

namespace Infrastructure.TaskJobs
{
    internal class LogCleanupJob : IJob
    {
        private const int RetentionDays = 10;

        public static readonly JobKey JobKey = new(nameof(LogCleanupJob));
        public ValueTask Execute(IJobExecutionContext context)
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

                Console.WriteLine($"Log cleanup completed. Deleted files older than {RetentionDays} days.");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Log cleanup failed: {ex.Message}");
            }

            return ValueTask.CompletedTask;
        }
    }
}
