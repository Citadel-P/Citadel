using Application.Configs;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal interface IBackupPolicyScheduler
{
    Task QueueDueScheduledRunsAsync(CancellationToken cancellationToken);
}

internal sealed class BackupPolicyScheduler(
    IServiceScopeFactory scopeFactory,
    IOptions<BackupOptions> backupOptions,
    IBackupRunStreamManager backupRunStreamManager,
    INotificationQueue notificationQueue,
    TimeProvider timeProvider,
    ILogger<BackupPolicyScheduler> logger) : IBackupPolicyScheduler
{
    private readonly BackupOptions options = backupOptions.Value;

    public async Task QueueDueScheduledRunsAsync(CancellationToken cancellationToken)
    {
        if (!options.Enabled)
            return;

        var nowUtc = SchedulerTime.TruncateToMinute(timeProvider.GetUtcNow());
        var policies = await GetScheduledPoliciesAsync(nowUtc, cancellationToken);

        foreach (var policy in policies)
        {
            try
            {
                if (!CronSchedule.IsDue(policy.Cron, policy.TimeZone, nowUtc.UtcDateTime))
                    continue;

                var queueResult = await QueueScheduledRunAsync(policy.Id, nowUtc, cancellationToken);
                if (queueResult.Status != BackupRunQueueResultStatus.Queued)
                {
                    logger.LogDebug(
                        "Skipped scheduled backup policy {BackupPolicyId}: {Status}",
                        policy.Id,
                        queueResult.Status);
                }
            }
            catch (OperationCanceledException)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Failed to queue scheduled backup policy {BackupPolicyId}", policy.Id);
            }
        }
    }

    private async Task<ScheduledBackupPolicy[]> GetScheduledPoliciesAsync(
        DateTimeOffset nowUtc,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await unitOfWork.BackupPolicies.GetScheduledAsync(nowUtc, cancellationToken)).ToArray();
    }

    private async Task<BackupRunQueueResult> QueueScheduledRunAsync(
        Guid policyId,
        DateTimeOffset nowUtc,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queueResult = await unitOfWork.BackupRuns.QueueScheduledAsync(
            policyId,
            Guid.CreateVersion7(),
            nowUtc,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        if (queueResult.Status == BackupRunQueueResultStatus.Queued)
        {
            await notificationQueue.EnqueueAsync(
                new BackupRunNotificationWorkItem(backupRunStreamManager, queueResult.Run!, "create"),
                cancellationToken);
        }

        return queueResult;
    }

}

internal sealed class BackupPolicySchedulerJob(
    IBackupPolicyScheduler scheduler,
    IOptions<BackupOptions> backupOptions,
    TimeProvider timeProvider,
    ILogger<BackupPolicySchedulerJob> logger) : BackgroundService
{
    private readonly BackupOptions options = backupOptions.Value;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        using var timer = new PeriodicTimer(
            TimeSpan.FromSeconds(Math.Max(5, options.SchedulePollIntervalSeconds)),
            timeProvider);

        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                try
                {
                    await scheduler.QueueDueScheduledRunsAsync(stoppingToken);
                }
                catch (OperationCanceledException)
                {
                    throw;
                }
                catch (Exception ex)
                {
                    logger.LogWarning(ex, "Backup policy scheduler tick failed.");
                }
            }
        }
        catch (OperationCanceledException)
        {
        }
    }
}
