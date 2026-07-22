using Application.Services.Builds;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class BuildAgentPoolHealthMonitorJob(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IBuildAgentPoolStreamManager streamManager,
    IDelayWithJitterService delayWithJitterService,
    ILogger<BuildAgentPoolHealthMonitorJob> logger) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan StableRefreshInterval = TimeSpan.FromMinutes(5);

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunAsync, cancellationToken: stoppingToken);

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await CheckPoolsAsync(cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during build agent pool health monitoring.");
            }

            await Task.Delay(CheckInterval, cancellationToken);
        }
    }

    internal async Task CheckPoolsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var validationService = scope.ServiceProvider.GetRequiredService<IBuildAgentPoolValidationService>();
        var pools = (await uow.BuildAgentPools.GetEnabledSelfManagedAsync(cancellationToken)).ToArray();
        if (pools.Length == 0)
            return;

        var updated = new List<BuildAgentPool>();
        foreach (var pool in pools)
        {
            if (pool.ControlState == ResourceControlState.Processing)
                continue;

            var validation = await validationService.ValidateAsync(pool, cancellationToken);
            var now = DateTimeOffset.UtcNow;
            if (!ShouldPersist(pool, validation.Status, validation.Message, now))
                continue;

            pool.ApplyValidation(validation.Status, validation.Message, now);
            await uow.BuildAgentPools.UpdateAsync(pool, cancellationToken);
            updated.Add(pool);
        }

        if (updated.Count == 0)
            return;

        await uow.CommitAsync(cancellationToken);
        foreach (var pool in updated)
        {
            await notificationQueue.EnqueueAsync(
                new BuildAgentPoolNotificationWorkItem(streamManager, pool),
                cancellationToken);
        }
    }

    private static bool ShouldPersist(
        BuildAgentPool pool,
        BuildAgentPoolValidationStatus status,
        string message,
        DateTimeOffset now)
    {
        if (pool.LastValidationStatus != status)
            return true;

        if (!string.Equals(pool.LastValidationMessage, message, StringComparison.Ordinal))
            return true;

        return pool.LastValidatedAt is null || now - pool.LastValidatedAt.Value > StableRefreshInterval;
    }
}
