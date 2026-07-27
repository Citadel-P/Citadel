using Application.Configs;
using Application.Services.Backups;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal sealed class BackupRestoreRunWorkerJob(
    IServiceScopeFactory scopeFactory,
    IOptions<BackupOptions> backupOptions,
    ILogger<BackupRestoreRunWorkerJob> logger) : BackgroundService
{
    private readonly BackupOptions options = backupOptions.Value;
    private readonly SemaphoreSlim concurrency = new(Math.Max(1, backupOptions.Value.MaxParallelRuns));
    private readonly TrackedBackgroundTasks activeTasks = new();

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var minimumDelay = TimeSpan.FromSeconds(Math.Max(1, options.PollIntervalSeconds));
        var delay = minimumDelay;

        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                var dispatched = options.Enabled
                    ? await DispatchQueuedRunsAsync(stoppingToken)
                    : 0;
                delay = WorkerPollingDelay.Next(delay, minimumDelay, dispatched > 0);
                await activeTasks.WaitForCompletionOrDelayAsync(delay, stoppingToken);
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        finally
        {
            await activeTasks.DrainAsync();
        }
    }

    private async Task<int> DispatchQueuedRunsAsync(CancellationToken stoppingToken)
    {
        var availableSlots = concurrency.CurrentCount;
        if (availableSlots <= 0)
            return 0;

        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queuedRunIds = await unitOfWork.BackupRestoreRuns.GetQueuedIdsAsync(
            Math.Min(options.MaxParallelRuns, availableSlots),
            stoppingToken);
        var dispatched = 0;

        foreach (var runId in queuedRunIds)
        {
            if (!concurrency.Wait(0))
                break;

            dispatched++;
            activeTasks.Add(ExecuteTrackedAsync(runId, stoppingToken));
        }

        return dispatched;
    }

    private async Task ExecuteTrackedAsync(Guid runId, CancellationToken stoppingToken)
    {
        try
        {
            await ExecuteRunAsync(runId, stoppingToken);
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Backup restore run {RunId} failed before execution service completed.", runId);
        }
        finally
        {
            concurrency.Release();
        }
    }

    private async Task ExecuteRunAsync(Guid runId, CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var executionService = scope.ServiceProvider.GetRequiredService<IBackupRestoreRunExecutionService>();
        await foreach (var _ in executionService.ExecuteQueuedAsync(runId, stoppingToken))
        {
        }
    }
}
