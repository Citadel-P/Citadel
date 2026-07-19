using Application.Configs;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services.Builds;

internal interface IBuildRunCleanupService
{
    Task<int> RemoveExpiredRunsAsync(IUnitOfWork unitOfWork, CancellationToken cancellationToken);
}

internal sealed class BuildRunCleanupService(
    IOptions<BuildOptions> options,
    TimeProvider timeProvider,
    ILogger<BuildRunCleanupService> logger) : IBuildRunCleanupService
{
    public async Task<int> RemoveExpiredRunsAsync(IUnitOfWork unitOfWork, CancellationToken cancellationToken)
    {
        var retentionDays = options.Value.RunRetentionDays;
        if (!options.Value.RunCleanupEnabled || retentionDays <= 0)
            return 0;

        var threshold = timeProvider.GetUtcNow().AddDays(-retentionDays).UtcDateTime;
        var deleted = await unitOfWork.BuildRuns.RemoveCompletedOlderThanAsync(threshold, cancellationToken);

        if (deleted > 0)
            logger.LogInformation("Purged {Count} build runs older than {RetentionDays} days.", deleted, retentionDays);

        return deleted;
    }
}
