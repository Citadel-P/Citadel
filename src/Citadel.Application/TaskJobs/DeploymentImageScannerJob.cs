using Application.Services;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DeploymentImageScannerJob(
    IImageScanScheduler imageScanScheduler,
    ISyncBarrier syncBarrier,
    IServiceScopeFactory scopeFactory,
    IDelayWithJitterService delayWithJitterService,
    IImageDigestScanner imageDigestScanner,
    ImageDigestCache imageDigestCache,
    ILogger<DeploymentImageScannerJob> logger) : BackgroundService
{
    private const int CheckIntervalInMinutes = 90;

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunPeriodicScanAsync, cancellationToken: stoppingToken);

    private async Task RunPeriodicScanAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var syncedPlatformIds = await LoadPlatformIdsAsync(cancellationToken);
                var scanTasks = await imageScanScheduler.LoadScanTasksAsync(cancellationToken);

                foreach (var scanTask in scanTasks)
                {
                    try
                    {
                        var result = await imageDigestScanner.ScanAsync(scanTask, cancellationToken);
                        if (result.IsFailure(out var error, out var digest))
                        {
                            logger.LogWarning(
                                "Image scan failed for {ImageKey}: {Error}",
                                scanTask.Key,
                                error.Message);
                            continue;
                        }

                        imageDigestCache.Set(scanTask.Key, digest);
                    }
                    catch (Exception ex)
                    {
                        logger.LogError(ex, "Scan failed for image {ImageKey}", scanTask.Key);
                    }
                }

                foreach (var platformId in syncedPlatformIds)
                {
                    syncBarrier.MarkSynced<DeploymentImageScannerJob>(platformId);
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Image scan failed.");
            }

            await Task.Delay(TimeSpan.FromMinutes(CheckIntervalInMinutes), cancellationToken);
        }
    }

    private async Task<Guid[]> LoadPlatformIdsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        return [.. (await uow.Platforms.GetPlatformsInfoAsync(cancellationToken))
            .Select(x => x.Id)
            .Distinct()];
    }

}
