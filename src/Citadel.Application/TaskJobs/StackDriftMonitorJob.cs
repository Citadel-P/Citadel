using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

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
    ILicenseEntitlementService entitlementService,
    IContainerEventBroadcaster containerEventBroadcaster,
    ILogger<StackDriftMonitorJob> logger) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromMinutes(5);

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunAsync, cancellationToken: stoppingToken);

    private async Task RunAsync(CancellationToken cancellationToken)
    {
        var containerEvents = containerEventBroadcaster.AddSubscriber();

        try
        {
            await RunCheckOnce(cancellationToken);

            using var timer = new PeriodicTimer(CheckInterval);
            var timerTask = timer.WaitForNextTickAsync(cancellationToken).AsTask();
            var eventTask = containerEvents.WaitToReadAsync(cancellationToken).AsTask();

            while (!cancellationToken.IsCancellationRequested)
            {
                await Task.WhenAny(timerTask, eventTask);

                if (timerTask.IsCompleted)
                {
                    if (!await timerTask)
                        break;

                    await RunCheckOnce(cancellationToken);
                    timerTask = timer.WaitForNextTickAsync(cancellationToken).AsTask();
                }

                if (eventTask.IsCompleted)
                {
                    if (!await eventTask)
                        break;

                    while (containerEvents.TryRead(out var containerEvent))
                    {
                        await HandleContainerEventSafelyAsync(containerEvent, cancellationToken);
                    }

                    eventTask = containerEvents.WaitToReadAsync(cancellationToken).AsTask();
                }
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        finally
        {
            containerEventBroadcaster.RemoveSubscriber(containerEvents);
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
        if (!await entitlementService.IsEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken))
            return;

        var stacks = await GetDriftMonitorStacks(cancellationToken);

        foreach (var stack in stacks)
        {
            await CheckStackSafelyAsync(stack, cancellationToken);
        }
    }

    private async Task HandleContainerEventSafelyAsync(
        ContainerEvent containerEvent,
        CancellationToken cancellationToken)
    {
        try
        {
            await HandleContainerEventAsync(containerEvent, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogWarning(
                ex,
                "Stack drift event check failed for container {ContainerId} on platform {PlatformId}",
                containerEvent.ContainerId,
                containerEvent.PlatformId);
        }
    }

    internal async Task HandleContainerEventAsync(
        ContainerEvent containerEvent,
        CancellationToken cancellationToken)
    {
        if (!IsDriftEvent(containerEvent.Action)
            || !await entitlementService.IsEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken))
        {
            return;
        }

        var stack = await GetAutoFixStackAsync(containerEvent, cancellationToken);
        if (stack is not null)
        {
            await CheckStackSafelyAsync(stack, cancellationToken);
        }
    }

    private async Task CheckStackSafelyAsync(
        StackDriftStack stack,
        CancellationToken cancellationToken)
    {
        try
        {
            await CheckStackAsync(stack, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Stack drift check failed for stack {StackId}", stack.Id);
        }
    }

    private async Task CheckStackAsync(
        StackDriftStack stack,
        CancellationToken cancellationToken)
    {
        var report = await driftChecker.CheckAsync(stack, cancellationToken);
        var fingerprint = StackDriftHelpers.Fingerprint(report);

        if (!report.HasDrift)
        {
            if (stack.Status == StackReleaseStatus.Degraded)
            {
                await PersistDriftStateAsync(stack.Id, report, fingerprint, cancellationToken);
            }

            return;
        }

        await PersistDriftStateAsync(stack.Id, report, fingerprint, cancellationToken);

        if (stack.DriftPolicy.Mode != StackDriftMode.AutoFix)
        {
            if (stack.DriftPolicy.AlertOnDrift)
            {
                await EmitAlertAsync(stack, report, fingerprint, cancellationToken);
            }

            return;
        }

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

        if (!stack.DriftPolicy.AlertOnDrift)
            return;

        if (result.Status == StackReconciliationStatus.Reconciled)
        {
            await EmitAutoReconciledAlertAsync(stack, report, fingerprint, result, cancellationToken);
        }
        else
        {
            await EmitAlertAsync(stack, report, fingerprint, cancellationToken);
        }
    }

    private async Task<List<StackDriftStack>> GetDriftMonitorStacks(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stacks = await unitOfWork.Stacks.GetDriftMonitorStacksAsync(cancellationToken);
        return [.. stacks];
    }

    private async Task<StackDriftStack?> GetAutoFixStackAsync(
        ContainerEvent containerEvent,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = await unitOfWork.Containers.GetContainerInfoAsync(
            containerEvent.ContainerId,
            cancellationToken);

        if (container?.StackId is not { } stackId
            || container.PlatformId != containerEvent.PlatformId
            || !IsStoppedOrPaused(container.State))
        {
            return null;
        }

        var stack = await unitOfWork.Stacks.GetDriftStackAsync(stackId, cancellationToken);
        if (stack is null
            || stack.DriftPolicy.Mode != StackDriftMode.AutoFix
            || stack.ControlState != ResourceControlState.Idle
            || stack.Status != StackReleaseStatus.Healthy
                && stack.Status != StackReleaseStatus.Degraded)
        {
            return null;
        }

        return stack;
    }

    private static bool IsDriftEvent(string action)
        => action.Equals("die", StringComparison.OrdinalIgnoreCase)
            || action.Equals("pause", StringComparison.OrdinalIgnoreCase);

    private static bool IsStoppedOrPaused(ContainerStateStatus state)
        => state is ContainerStateStatus.Exited
            or ContainerStateStatus.Dead
            or ContainerStateStatus.Offline
            or ContainerStateStatus.Paused;

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

    private async Task EmitAutoReconciledAlertAsync(
        StackDriftStack stack,
        StackDriftReport report,
        string fingerprint,
        StackReconciliationResult result,
        CancellationToken cancellationToken)
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
                    Severity: AlertSeverity.Info,
                    Fingerprint: fingerprint,
                    DriftSummaries: StackDriftHelpers.Summaries(report),
                    ReconciliationResult: result)
            ]);

        await alertService.ProcessAsync(AlertType.StackDriftAutoReconciled, context, cancellationToken);
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
