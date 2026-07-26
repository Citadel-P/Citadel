using Application.Services;
using Application.Services.Alerts;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class ManualStackAutoUpdateJob(
    IDbWorkQueue dbWorkQueue,
    IAlertService alertService,
    IImageScanScheduler imageScanScheduler,
    ImageDigestCache imageDigestCache,
    ISyncBarrier syncBarrier,
    IServiceScopeFactory scopeFactory,
    IDelayWithJitterService delayWithJitterService,
    ILicenseEntitlementService entitlementService,
    ManualStackUpdateEvaluator updateEvaluator,
    ILogger<ManualStackAutoUpdateJob> logger) : BackgroundService
{
    private const int CheckIntervalInHours = 2;

    protected override Task ExecuteAsync(CancellationToken stoppingToken)
        => delayWithJitterService.DelayWithJitterForAsync(RunPeriodicAutoUpdate, cancellationToken: stoppingToken);

    private async Task RunPeriodicAutoUpdate(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                var checks = await imageScanScheduler.LoadManualStackChecksAsync(cancellationToken);
                foreach (var stackChecks in checks.GroupBy(check => check.Stack.Id))
                {
                    try
                    {
                        var firstCheck = stackChecks.First();
                        await syncBarrier.WaitForAsync<DeploymentImageScannerJob>(
                            firstCheck.Stack.CurrentStackRelease!.PlatformId,
                            cancellationToken);

                        var result = await CheckStackAsync(firstCheck.Stack, [.. stackChecks], cancellationToken);
                        if (result is not null)
                        {
                            await dbWorkQueue.EnqueueAsync(result, cancellationToken);
                        }
                    }
                    catch (Exception ex)
                    {
                        logger.LogError(ex, "Auto-update failed for stack {StackId}", stackChecks.Key);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while running manual stack auto-update job.");
            }

            await Task.Delay(TimeSpan.FromHours(CheckIntervalInHours), cancellationToken);
        }
    }

    private async Task<IDbWorkItem?> CheckStackAsync(
        Stack stack,
        IReadOnlyList<ManualStackImageCheck> checks,
        CancellationToken cancellationToken)
    {
        if (stack.CurrentStackRelease?.Spec is not ManualStack manualStack)
            return null;

        if (stack.StackUpdateState is not ManualStackUpdateState)
            return null;

        var now = DateTime.UtcNow;
        var digests = checks
            .Select(check => check.Key)
            .Distinct()
            .Select(key => imageDigestCache.TryGet(key, out var entry)
                ? new KeyValuePair<ImageKey, string>?(
                    new KeyValuePair<ImageKey, string>(key, entry.Digest))
                : null)
            .Where(item => item.HasValue)
            .Select(item => item!.Value)
            .ToDictionary(item => item.Key, item => item.Value);
        var evaluation = updateEvaluator.Evaluate(stack.StackUpdateState, checks, digests, now);
        var nextStates = evaluation.State.RecreateStackOnNewImageState.AutoUpdateStates;
        var updates = evaluation.NewlyDetectedUpdates;

        if (nextStates.Count == 0)
            return null;

        if (updates.Count == 0)
        {
            return new ManualStackAutoUpdateStateWorkItem(
                stack.Id,
                evaluation.State);
        }

        var canAutoUpdate = await entitlementService.IsEnabledAsync(
            LicenseCapability.OperationalGuardrails,
            cancellationToken);
        if (manualStack.UpdateBehavior == StackUpdateBehavior.Notify || !canAutoUpdate)
        {
            await ProcessAlertAsync(AlertType.StackImageUpdateAvailable, stack, updates, failed: false, null, cancellationToken);

            return new ManualStackAutoUpdateStateWorkItem(
                stack.Id,
                evaluation.State);
        }

        var serviceNames = manualStack.UpdateBehavior == StackUpdateBehavior.ServiceAutoDeploy
            ? updates.Select(update => update.ServiceName).Distinct(StringComparer.OrdinalIgnoreCase).ToArray()
            : null;

        var (success, error) = await TryAutoDeploy(stack.Id, serviceNames, cancellationToken);
        if (!success)
        {
            var reason = error ?? "Auto-deploy failed.";
            var alertType = manualStack.UpdateBehavior == StackUpdateBehavior.ServiceAutoDeploy
                ? AlertType.StackServiceAutoDeployFailed
                : AlertType.StackAutoDeployFailed;

            await ProcessAlertAsync(alertType, stack, updates, failed: true, reason, cancellationToken);

            return new ManualStackAutoUpdateStateWorkItem(
                stack.Id,
                evaluation.State);
        }

        var updatedStates = nextStates
            .Select(state => updates.Any(update =>
                string.Equals(update.ServiceName, state.ServiceName, StringComparison.OrdinalIgnoreCase)
                && string.Equals(update.ImageName, state.ImageName, StringComparison.OrdinalIgnoreCase))
                ? state with { CurrentDigest = state.RemoteDigest ?? state.CurrentDigest, UpdateAvailable = false }
                : state)
            .ToArray();

        var successType = manualStack.UpdateBehavior == StackUpdateBehavior.ServiceAutoDeploy
            ? AlertType.StackServiceAutoUpdated
            : AlertType.StackAutoUpdated;

        await ProcessAlertAsync(successType, stack, updates, failed: false, null, cancellationToken);

        return new ManualStackAutoUpdateStateWorkItem(
            stack.Id,
            new ManualStackUpdateState(new RecreateStackOnNewImageState(updatedStates)));
    }

    private async Task<(bool Success, string? Error)> TryAutoDeploy(
        Guid stackId,
        IReadOnlyList<string>? serviceNames,
        CancellationToken cancellationToken)
    {
        string? error = null;

        await using var scope = scopeFactory.CreateAsyncScope();
        var applyStackService = scope.ServiceProvider.GetRequiredService<IApplyStackService>();

        try
        {
            await foreach (var item in applyStackService.ApplyAsync(
                stackId,
                Constants.SystemId,
                serviceNames,
                pullImages: true,
                recreate: false,
                waitForCompletion: false,
                operation: StackApplyOperation.Apply,
                previousStackSnapshot: null,
                cancellationToken))
            {
                if (!string.IsNullOrEmpty(item.Message))
                {
                    error = item.Message;
                    break;
                }
            }
        }
        catch (Exception ex)
        {
            error = ex.Message;
        }

        return string.IsNullOrEmpty(error)
            ? (true, null)
            : (false, error);
    }

    private Task ProcessAlertAsync(
        AlertType type,
        Stack stack,
        IReadOnlyList<StackImageUpdateItem> updates,
        bool failed,
        string? reason,
        CancellationToken cancellationToken)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks:
            [
                new StackAlertSnapshot(
                    Id: stack.Id,
                    Name: stack.Name,
                    CurrentImage: updates.FirstOrDefault()?.LatestDigest ?? string.Empty,
                    PreviousImage: updates.FirstOrDefault()?.CurrentDigest ?? string.Empty,
                    LatestImage: updates.FirstOrDefault()?.LatestDigest ?? string.Empty,
                    Failed: failed,
                    Raison: reason,
                    Updates: updates,
                    ServiceNames: updates.Select(update => update.ServiceName).ToArray())
            ]);

        return alertService.ProcessAsync(type, context, cancellationToken);
    }

}

internal sealed class ManualStackAutoUpdateStateWorkItem(
    Guid stackId,
    ManualStackUpdateState state) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        await uow.Stacks.UpdateStackUpdateStateAsync(stackId, state, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}
