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
        var queued = await unitOfWork.ActionRuns.GetQueuedAsync(options.MaxParallelRuns, stoppingToken);

        foreach (var run in queued)
        {
            if (!concurrency.Wait(0))
                return;

            _ = Task.Run(async () =>
            {
                try
                {
                    await ClaimAndExecuteAsync(run.Id, stoppingToken);
                }
                catch (Exception ex) when (ex is not OperationCanceledException)
                {
                    logger.LogError(ex, "Automation run {RunId} failed before execution service completed", run.Id);
                }
                finally
                {
                    concurrency.Release();
                }
            }, stoppingToken);
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
