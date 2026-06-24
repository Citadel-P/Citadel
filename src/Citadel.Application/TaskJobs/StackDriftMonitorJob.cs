using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class StackDriftMonitorJob(
    IServiceScopeFactory scopeFactory,
    IDbWorkQueue dbQueue,
    IAlertService alertService,
    IStackDriftChecker driftChecker,
    IStackReconciler reconciler,
    INotificationQueue notificationQueue,
    IStackStreamManager stackStreamManager,
    IActivityStreamManager activityStreamManager,
    IDelayWithJitterService delayWithJitterService,
    ILogger<StackDriftMonitorJob> logger) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromMinutes(5);

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunAsync, cancellationToken: stoppingToken);

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        try
        {
            await RunCheckOnce(cancellationToken);

            using var timer = new PeriodicTimer(CheckInterval);
            while (await timer.WaitForNextTickAsync(cancellationToken))
            {
                await RunCheckOnce(cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
    }

    private async Task RunCheckOnce(CancellationToken cancellationToken)
    {
        try
        {
            await CheckStacksAsync(cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred while running stack drift monitor.");
        }
    }

    private async Task CheckStacksAsync(CancellationToken cancellationToken)
    {
        var stacks = await GetDriftMonitorStacks(cancellationToken);

        foreach (var stack in stacks)
        {
            try
            {
                var report = await driftChecker.CheckAsync(stack, cancellationToken);
                var fingerprint = StackDriftHelpers.Fingerprint(report);

                if (report.HasDrift)
                {
                    await PersistDriftStateAsync(stack.Id, report, fingerprint, cancellationToken);

                    if (stack.DriftPolicy.AlertOnDrift)
                    {
                        await EmitAlertAsync(stack, report, fingerprint, cancellationToken);
                    }

                    if (stack.DriftPolicy.Mode == StackDriftMode.AutoFix)
                    {
                        var result = await reconciler.ReconcileAsync(stack.Id, cancellationToken);

                        await dbQueue.EnqueueAndWaitAsync(new StackReconciliationResultWorkItem(
                            stack.Id,
                            result,
                            fingerprint,
                            notificationQueue,
                            activityStreamManager), cancellationToken);

                        if (result.AfterReport is not null)
                        {
                            await PersistDriftStateAsync(
                                stack.Id,
                                result.AfterReport,
                                StackDriftHelpers.Fingerprint(result.AfterReport),
                                fingerprint,
                                cancellationToken);
                        }
                    }
                }
                else
                {
                    if (stack.Status != StackReleaseStatus.Degraded)
                    {
                        continue;
                    }

                    await PersistDriftStateAsync(stack.Id, report, fingerprint, cancellationToken);
                }
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Stack drift check failed for stack {StackId}", stack.Id);
            }
        }
    }

    private async Task<List<StackDriftStack>> GetDriftMonitorStacks(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stacks = await unitOfWork.Stacks.GetDriftMonitorStacksAsync(cancellationToken);
        return [.. stacks];
    }

    private ValueTask PersistDriftStateAsync(
        Guid stackId,
        StackDriftReport report,
        string fingerprint,
        string? previousDriftFingerprint,
        CancellationToken cancellationToken)
        => dbQueue.EnqueueAndWaitAsync(new StackDriftStatusWorkItem(
            stackId,
            report,
            fingerprint,
            notificationQueue,
            stackStreamManager,
            activityStreamManager,
            previousDriftFingerprint), cancellationToken);

    private ValueTask PersistDriftStateAsync(
        Guid stackId,
        StackDriftReport report,
        string fingerprint,
        CancellationToken cancellationToken)
        => PersistDriftStateAsync(stackId, report, fingerprint, previousDriftFingerprint: null, cancellationToken);

    private async Task EmitAlertAsync(StackDriftStack stack, StackDriftReport report, string fingerprint, CancellationToken cancellationToken)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            StackDrifts:
            [
                new StackDriftAlertSnapshot(
                    Id: stack.Id,
                    Name: stack.Name,
                    PlatformId: report.PlatformId,
                    PlatformName: stack.PlatformName ?? string.Empty,
                    Report: report,
                    Severity: StackDriftHelpers.GetSeverity(report),
                    Fingerprint: fingerprint,
                    DriftSummaries: StackDriftHelpers.Summaries(report))
            ]);

        await alertService.ProcessAsync(AlertType.StackDriftDetected, context, cancellationToken);
    }
}

internal sealed class StackDriftStatusWorkItem(
    Guid stackId,
    StackDriftReport report,
    string fingerprint,
    INotificationQueue notificationQueue,
    IStackStreamManager stackStreamManager,
    IActivityStreamManager activityStreamManager,
    string? previousDriftFingerprint = null) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var stack = await uow.Stacks.GetAsync(stackId, cancellationToken);
        if (stack?.CurrentStackRelease is null)
            return;

        var stackChanged = false;
        ActivityEvent? activity = null;

        if (!report.HasDrift)
        {
            var latestDriftFingerprint = previousDriftFingerprint ?? GetLatestDriftFingerprint(stack.LatestActivityEvent?.Info);
            if (stack.CurrentStackRelease.Status == StackReleaseStatus.Degraded
                && latestDriftFingerprint is not null)
            {
                stack.PartialUpdate(StackReleaseStatus.Healthy);
                stackChanged = true;
                activity = new ActivityEvent(
                    actorId: Constants.SystemId,
                    resourceId: stack.Id,
                    platformId: stack.CurrentStackRelease.PlatformId,
                    resourceName: stack.Name,
                    eventType: ActivityEventType.StackDriftResolved,
                    status: ActivityStatus.Success,
                    info: new StackDriftResolved(latestDriftFingerprint));

                await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
                stack.AssignActivityEvent(activity);
            }
        }
        else
        {
            var reason = BuildReason(report);
            var latestDriftFingerprint = GetLatestDriftFingerprint(stack.LatestActivityEvent?.Info);
            var shouldWriteActivity = latestDriftFingerprint != fingerprint;

            if (stack.DriftPolicy.MarkDegraded && stack.CurrentStackRelease.Status != StackReleaseStatus.Degraded)
            {
                stack.PartialUpdate(StackReleaseStatus.Degraded);
                stackChanged = true;
            }

            if (shouldWriteActivity)
            {
                activity = new ActivityEvent(
                    actorId: Constants.SystemId,
                    resourceId: stack.Id,
                    platformId: stack.CurrentStackRelease.PlatformId,
                    resourceName: stack.Name,
                    eventType: ActivityEventType.StackDriftDetected,
                    status: ActivityStatus.Warning,
                    info: new StackDriftDetected(reason, fingerprint));

                await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
                stack.AssignActivityEvent(activity);
            }
        }

        if (!stackChanged && activity is null)
            return;

        if (stackChanged)
        {
            await uow.Stacks.UpdateAsync(stack, cancellationToken);
        }

        await uow.CommitAsync(cancellationToken);

        if (stackChanged)
        {
            await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackStreamManager, stack), cancellationToken);
        }

        if (activity is not null)
        {
            await notificationQueue.EnqueueAsync(
                new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(uow, cancellationToken)),
                cancellationToken);
        }
    }

    private static string? GetLatestDriftFingerprint(ActivityEventInfo? info)
        => info switch
        {
            StackDriftDetected driftDetected => driftDetected.Fingerprint,
            _ => null
        };

    private static string BuildReason(StackDriftReport report)
    {
        var issueText = report.Drifts.Count == 1
            ? "1 issue"
            : $"{report.Drifts.Count} issues";

        var summary = StackDriftHelpers.ShortSummary(report);
        return string.IsNullOrWhiteSpace(summary)
            ? $"Drift detected: {issueText}."
            : $"Drift detected: {issueText}. {summary}";
    }
}

internal sealed class StackReconciliationResultWorkItem(
    Guid stackId,
    StackReconciliationResult result,
    string driftFingerprint,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityStreamManager) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        if (result.Actions.Count == 0)
        {
            return;
        }

        var stack = await uow.Stacks.GetAsync(stackId, cancellationToken);
        if (stack?.CurrentStackRelease is null)
            return;

        var activity = new ActivityEvent(
            actorId: Constants.SystemId,
            resourceId: stack.Id,
            platformId: stack.CurrentStackRelease.PlatformId,
            resourceName: stack.Name,
            eventType: ActivityEventType.StackReconciliationAttempted,
            status: GetActivityStatus(result.Status),
            info: new StackReconciliationAttempted(result.Status, result.Actions, driftFingerprint));

        await uow.ActivityEventRepository.AddAsync(activity, cancellationToken);
        stack.AssignActivityEvent(activity);
        await uow.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(uow, cancellationToken)),
            cancellationToken);
    }

    private static ActivityStatus GetActivityStatus(StackReconciliationStatus status)
        => status switch
        {
            StackReconciliationStatus.Reconciled or StackReconciliationStatus.NoDrift => ActivityStatus.Success,
            StackReconciliationStatus.Failed => ActivityStatus.Failure,
            _ => ActivityStatus.Warning
        };
}
