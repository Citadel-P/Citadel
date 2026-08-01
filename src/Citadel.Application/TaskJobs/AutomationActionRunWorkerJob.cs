using Application.Configs;
using Application.Services;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal sealed class AutomationActionRunWorkerJob(
    IServiceScopeFactory scopeFactory,
    IOptions<AutomationOptions> automationOptions,
    ILogger<AutomationActionRunWorkerJob> logger) : BackgroundService
{
    private readonly AutomationOptions options = automationOptions.Value;
    private readonly SemaphoreSlim concurrency = new(Math.Max(1, automationOptions.Value.MaxParallelRuns));
    private readonly TrackedBackgroundTasks activeTasks = new();
    private CancellationToken shutdownDeadline;

    public override Task StopAsync(CancellationToken cancellationToken)
    {
        shutdownDeadline = cancellationToken;
        return base.StopAsync(cancellationToken);
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var minimumDelay = TimeSpan.FromSeconds(Math.Max(1, options.PollIntervalSeconds));
        var delay = minimumDelay;

        try
        {
            await ReconcileInterruptedRunsAsync(minimumDelay, stoppingToken);

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

    private async Task ReconcileInterruptedRunsAsync(
        TimeSpan retryDelay,
        CancellationToken cancellationToken)
    {
        while (true)
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var interrupted = await unitOfWork.ActionRuns.InterruptInProgressAsync(
                    DateTime.UtcNow,
                    "Automation run was interrupted by an application restart.",
                    cancellationToken);
                await unitOfWork.AutomationActions.ResetActionsWithTerminalRunsAsync(cancellationToken);
                await unitOfWork.CommitAsync(cancellationToken);

                if (interrupted > 0)
                {
                    logger.LogWarning(
                        "Marked {Count} in-progress automation runs as failed after application restart.",
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
                    "Citadel could not reconcile interrupted automation runs during startup. Retrying.");
                await Task.Delay(retryDelay, cancellationToken);
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
        var queued = await unitOfWork.ActionRuns.GetQueuedAsync(
            Math.Min(options.MaxParallelRuns, availableSlots),
            stoppingToken);
        var dispatched = 0;

        foreach (var run in queued)
        {
            if (!concurrency.Wait(0))
                break;

            dispatched++;
            activeTasks.Add(ExecuteTrackedAsync(run.Id, stoppingToken));
        }

        return dispatched;
    }

    private async Task ExecuteTrackedAsync(Guid runId, CancellationToken stoppingToken)
    {
        try
        {
            await ClaimAndExecuteAsync(runId, stoppingToken);
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Automation run {RunId} failed before execution service completed", runId);
        }
        finally
        {
            concurrency.Release();
        }
    }

    private async Task ClaimAndExecuteAsync(Guid runId, CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var executionService = scope.ServiceProvider.GetRequiredService<IAutomationExecutionService>();
        await foreach (var _ in executionService.ExecuteQueuedAsync(runId, stoppingToken))
        {
        }
    }
}
