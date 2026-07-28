using Application.Configs;
using Application.Services.Builds;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal sealed class BuildRunWorkerJob(
    IServiceScopeFactory scopeFactory,
    IOptions<BuildOptions> buildOptions,
    ILogger<BuildRunWorkerJob> logger)
    : BackgroundService
{
    private static readonly TimeSpan Delay = TimeSpan.FromSeconds(5);
    private readonly BuildOptions options = buildOptions.Value;
    private readonly SemaphoreSlim concurrency = new(Math.Max(1, buildOptions.Value.MaxParallelRuns));
    private readonly TrackedBackgroundTasks activeTasks = new();

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                try
                {
                    await DispatchQueuedRunsAsync(stoppingToken);
                }
                catch (Exception ex) when (ex is not OperationCanceledException || !stoppingToken.IsCancellationRequested)
                {
                    logger.LogError(ex, "Build run worker failed while reading queued runs.");
                }

                await activeTasks.WaitForCompletionOrDelayAsync(Delay, stoppingToken);
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        finally
        {
            await activeTasks.DrainAsync();
        }
    }

    private async Task DispatchQueuedRunsAsync(CancellationToken stoppingToken)
    {
        var availableSlots = concurrency.CurrentCount;
        if (availableSlots <= 0)
            return;

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queued = await uow.BuildRuns.GetQueuedAsync(
            Math.Min(options.MaxParallelRuns, availableSlots),
            stoppingToken);

        foreach (var run in queued)
        {
            if (!concurrency.Wait(0))
                break;

            activeTasks.Add(ExecuteTrackedAsync(run.Id, stoppingToken));
        }
    }

    private async Task ExecuteTrackedAsync(Guid runId, CancellationToken stoppingToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var executionService = scope.ServiceProvider.GetRequiredService<IBuildRunExecutionService>();
            await executionService.ExecuteAsync(runId, stoppingToken);
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        catch (Exception ex)
        {
            logger.LogError(ex, "Build run {RunId} failed before execution completed.", runId);
        }
        finally
        {
            concurrency.Release();
        }
    }
}
