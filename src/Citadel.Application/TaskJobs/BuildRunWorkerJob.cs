using Application.Features.Builds.Commands;
using Domain.Contracts.Interfaces;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class BuildRunWorkerJob(
    IServiceScopeFactory scopeFactory,
    ILogger<BuildRunWorkerJob> logger)
    : BackgroundService
{
    private static readonly TimeSpan Delay = TimeSpan.FromSeconds(5);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                await using var scope = scopeFactory.CreateAsyncScope();
                var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
                var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();
                var queued = await uow.BuildRuns.GetQueuedAsync(5, stoppingToken);

                foreach (var run in queued)
                    await mediator.Send(new ExecuteQueuedBuildRun(run.Id), stoppingToken);
            }
            catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Build run worker failed.");
            }

            await Task.Delay(Delay, stoppingToken);
        }
    }
}
