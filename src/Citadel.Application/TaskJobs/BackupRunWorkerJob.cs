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
    ICitadelSystemBackupBuilder citadelSystemBackupBuilder,
    TimeProvider timeProvider,
    IOptions<BackupOptions> backupOptions,
    ILogger<BackupRunWorkerJob> logger) : BackgroundService
{
    private readonly BackupOptions options = backupOptions.Value;
    private readonly SemaphoreSlim concurrency = new(Math.Max(1, backupOptions.Value.MaxParallelRuns));
    private readonly TrackedBackgroundTasks activeTasks = new();
    private CancellationToken shutdownDeadline;

    public override Task StopAsync(CancellationToken cancellationToken)
    {
        shutdownDeadline = cancellationToken;
        return base.StopAsync(cancellationToken);
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await InterruptAbandonedRunsAsync(stoppingToken);

        var removedStaleBundles = citadelSystemBackupBuilder.CleanupStaleBundles();
        if (removedStaleBundles > 0)
        {
            logger.LogInformation(
                "Deleted {Count} stale Citadel recovery bundle staging directories.",
                removedStaleBundles);
        }

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
            await activeTasks.DrainAsync(shutdownDeadline);
        }
    }

    private async Task InterruptAbandonedRunsAsync(CancellationToken cancellationToken)
    {
        while (true)
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var interrupted = await unitOfWork.BackupRuns.InterruptInProgressAsync(
                    timeProvider.GetUtcNow(),
                    "Backup run was interrupted by an application restart.",
                    cancellationToken);
                await unitOfWork.CommitAsync(cancellationToken);

                if (interrupted > 0)
                {
                    logger.LogWarning(
                        "Marked {Count} in-progress backup runs as interrupted after application restart.",
                        interrupted);
                }

                return;
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Citadel could not reconcile interrupted backup runs during startup. Retrying.");
                await Task.Delay(
                    TimeSpan.FromSeconds(Math.Max(1, options.PollIntervalSeconds)),
                    cancellationToken);
            }
        }
    }

    private async Task<int> DispatchQueuedRunsAsync(CancellationToken stoppingToken)
    {
        var availableSlots = concurrency.CurrentCount;
        if (availableSlots <= 0)
            return 0;

        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queuedRunIds = await unitOfWork.BackupRuns.GetQueuedIdsAsync(
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
            logger.LogError(ex, "Backup run {RunId} failed before execution service completed.", runId);
        }
        finally
        {
            concurrency.Release();
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
