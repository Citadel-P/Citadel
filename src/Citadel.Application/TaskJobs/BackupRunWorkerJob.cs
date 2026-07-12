using Application.Configs;
using Application.Services.Backups;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal sealed class BackupRunWorkerJob(
    IServiceScopeFactory scopeFactory,
    IOptions<BackupOptions> backupOptions,
    ILogger<BackupRunWorkerJob> logger) : BackgroundService
{
    private readonly BackupOptions options = backupOptions.Value;
    private readonly SemaphoreSlim concurrency = new(Math.Max(1, backupOptions.Value.MaxParallelRuns));

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        using var timer = new PeriodicTimer(TimeSpan.FromSeconds(Math.Max(1, options.PollIntervalSeconds)));

        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                if (!options.Enabled)
                    continue;

                await DispatchQueuedRunsAsync(stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private async Task DispatchQueuedRunsAsync(CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queuedRunIds = await unitOfWork.BackupRuns.GetQueuedIdsAsync(options.MaxParallelRuns, stoppingToken);

        foreach (var runId in queuedRunIds)
        {
            if (!concurrency.Wait(0))
                return;

            _ = Task.Run(async () =>
            {
                try
                {
                    await ExecuteRunAsync(runId, stoppingToken);
                }
                catch (Exception ex) when (ex is not OperationCanceledException)
                {
                    logger.LogError(ex, "Backup run {RunId} failed before execution service completed.", runId);
                }
                finally
                {
                    concurrency.Release();
                }
            }, stoppingToken);
        }
    }

    private async Task ExecuteRunAsync(Guid runId, CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var executionService = scope.ServiceProvider.GetRequiredService<IBackupRunExecutionService>();
        await foreach (var _ in executionService.ExecuteQueuedAsync(runId, stoppingToken))
        {
        }
    }
}
