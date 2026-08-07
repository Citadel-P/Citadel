using Application.Services;
using Application.Services.Alerts;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DeploymentAutoUpdateJob(
    IDbWorkQueue dbWorkQueue,
    IAlertService alertService,
    IImageScanScheduler imageScanScheduler,
    ImageDigestCache imageDigestCache,
    ISyncBarrier syncBarrier,
    IServiceScopeFactory scopeFactory,
    IDelayWithJitterService delayWithJitterService,
    ILicenseEntitlementService entitlementService,
    DeploymentUpdateEvaluator updateEvaluator,
    ISwarmServiceStreamManager swarmServiceStreamManager,
    INotificationQueue notificationQueue,
    ILogger<DeploymentAutoUpdateJob> logger) : BackgroundService
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
                await RunOnceAsync(cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while running deployment auto-update job.");
            }

            await Task.Delay(TimeSpan.FromHours(CheckIntervalInHours), cancellationToken);
        }
    }

    internal async Task RunOnceAsync(CancellationToken cancellationToken)
    {
        var deploymentChecks = await imageScanScheduler.LoadDeploymentChecksAsync(cancellationToken);
        foreach (var deploymentCheck in deploymentChecks)
        {
            try
            {
                await syncBarrier.WaitForAsync<DeploymentImageScannerJob>(
                    deploymentCheck.Deployment.PlatformId,
                    cancellationToken);

                var result = await CheckDeploymentAsync(deploymentCheck, cancellationToken);
                if (result is not null)
                    await dbWorkQueue.EnqueueAsync(result, cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Auto-update failed for deployment {DeploymentId}",
                    deploymentCheck.Deployment.Id);
            }
        }

        var serviceChecks = await imageScanScheduler.LoadSwarmServiceChecksAsync(cancellationToken);
        foreach (var serviceCheck in serviceChecks)
        {
            try
            {
                await syncBarrier.WaitForAsync<DeploymentImageScannerJob>(
                    serviceCheck.Service.PlatformId,
                    cancellationToken);
                await CheckSwarmServiceAsync(serviceCheck, cancellationToken);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                throw;
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Auto-update failed for managed Swarm Service {ServiceId}",
                    serviceCheck.Service.Id);
            }
        }
    }

    private async Task CheckSwarmServiceAsync(
        SwarmServiceImageCheck check,
        CancellationToken cancellationToken)
    {
        if (!imageDigestCache.TryGet(check.Key, out var digestEntry)
            || string.IsNullOrWhiteSpace(check.Service.AppliedImageDigest))
            return;

        var currentDigest = check.Service.AppliedImageDigest;
        var remoteDigest = digestEntry.Digest;
        var now = DateTime.UtcNow;
        var evaluation = updateEvaluator.Evaluate(currentDigest, remoteDigest, now);

        if (!evaluation.UpdateAvailable
            || check.Service.Spec.UpdateBehavior == UpdateBehavior.Notify
            || !await entitlementService.IsEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken))
        {
            await QueueSwarmServiceStateAsync(check, evaluation.State, cancellationToken);
            return;
        }

        var applied = await TryAutoDeploySwarmServiceAsync(check, currentDigest, cancellationToken);
        if (!applied.IsSuccess(out var service, out var error))
        {
            await QueueSwarmServiceStateAsync(
                check,
                new AutoUpdateState(
                    now,
                    AutoUpdateStatus.Failed,
                    currentDigest,
                    remoteDigest,
                    error!.Message),
                cancellationToken);
            return;
        }

        var appliedDigest = service.AppliedImageDigest;
        var state = string.Equals(appliedDigest, remoteDigest, StringComparison.OrdinalIgnoreCase)
            ? new AutoUpdateState(now, AutoUpdateStatus.UpToDate, appliedDigest, remoteDigest)
            : evaluation.State;
        await QueueSwarmServiceStateAsync(check, state, cancellationToken);
    }

    private async Task<Result<SwarmService>> TryAutoDeploySwarmServiceAsync(
        SwarmServiceImageCheck check,
        string expectedAppliedDigest,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var current = await unitOfWork.SwarmServices.GetAsync(check.Service.Id, cancellationToken);
        if (current is null)
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service no longer exists."));
        if (current.Spec.UpdateBehavior != UpdateBehavior.AutoDeploy
            || current.Spec.Image is not SwarmExternalImage image
            || image.RegistryId != check.Key.RegistryId
            || !Helpers.TrySplitImageTag(image.ImageTag, out var repository, out var tag)
            || !string.Equals(repository, check.Key.Repository, StringComparison.OrdinalIgnoreCase)
            || !string.Equals(tag, check.Key.Tag, StringComparison.Ordinal)
            || !string.Equals(current.AppliedImageDigest, expectedAppliedDigest, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<SwarmService>(new ConflictError(
                "The Service update policy or image changed before AutoDeploy started."));
        }

        var mutationService = scope.ServiceProvider.GetRequiredService<ISwarmServiceMutationService>();
        return await mutationService.ApplyAsync(current.Id, Constants.SystemId, cancellationToken);
    }

    private ValueTask QueueSwarmServiceStateAsync(
        SwarmServiceImageCheck check,
        AutoUpdateState state,
        CancellationToken cancellationToken) =>
        dbWorkQueue.EnqueueAsync(
            new SwarmServiceAutoUpdateStateWorkItem(
                check.Service.Id,
                check.Key,
                state,
                swarmServiceStreamManager,
                notificationQueue),
            cancellationToken);

    private async Task<IDbWorkItem?> CheckDeploymentAsync(
        DeploymentImageCheck deploymentCheck,
        CancellationToken cancellationToken)
    {
        var deployment = deploymentCheck.Deployment;
        var deployedImage = deploymentCheck.DeployedImage;

        if (!imageDigestCache.TryGet(deploymentCheck.Key, out var digestEntry))
            return null;

        if (string.IsNullOrEmpty(deployedImage.ResolvedDigest))
            return null;

        var remoteDigest = digestEntry.Digest;
        var currentDigest = deployedImage.ResolvedDigest;
        var now = DateTime.UtcNow;
        var evaluation = updateEvaluator.Evaluate(currentDigest, remoteDigest, now);

        if (!evaluation.UpdateAvailable)
        {
            return new DeploymentAutoUpdateStateWorkItem(
                deployment.Id,
                evaluation.State
            );
        }

        var canAutoUpdate = await entitlementService.IsEnabledAsync(
            LicenseCapability.OperationalGuardrails,
            cancellationToken);
        if (deployment.Spec?.UpdateBehavior == UpdateBehavior.Notify || !canAutoUpdate)
        {
            var context = new AlertEvaluationContext(
                UtcNow: now,
                Platforms: [],
                Deployments:
                [
                    new DeploymentAlertSnapshot(
                        Id: deployment.Id,
                        Name: deployment.Name,
                        CurrentImage: currentDigest,
                        PreviousImage: null,
                        LatestImage: remoteDigest,
                        Failed: false)
                ],
                Stacks: []);

            await alertService.ProcessAsync(
                AlertType.DeploymentImageUpdateAvailable,
                context,
                cancellationToken);

            logger.LogInformation(
                "Auto-update available for deployment {DeploymentId}: new digest detected.",
                deployment.Id);

            return new DeploymentAutoUpdateStateWorkItem(
                deployment.Id,
                evaluation.State
            );
        }
        else
        {
            var (Success, Error) = await TryAutoDeploy(deployment.Id, cancellationToken);

            if (!Success)
            {
                var reason = Error ?? "Auto-deploy failed.";

                var failedContext = new AlertEvaluationContext(
                    UtcNow: now,
                    Platforms: [],
                    Deployments:
                    [
                        new DeploymentAlertSnapshot(
                            Id: deployment.Id,
                            Name: deployment.Name,
                            CurrentImage: currentDigest,
                            PreviousImage: currentDigest,
                            LatestImage: remoteDigest,
                            Failed: true,
                            Raison: reason)
                    ],
                    Stacks: []);

                await alertService.ProcessAsync(
                    AlertType.DeploymentAutoDeployFailed,
                    failedContext,
                    cancellationToken);

                return new DeploymentAutoUpdateFailedWorkItem(deployment.Id, reason);
            }

            var updatedContext = new AlertEvaluationContext(
                UtcNow: now,
                Platforms: [],
                Deployments:
                [
                    new DeploymentAlertSnapshot(
                        Id: deployment.Id,
                        Name: deployment.Name,
                        CurrentImage: remoteDigest,
                        PreviousImage: currentDigest,
                        LatestImage: remoteDigest,
                        Failed: false)
                ],
                Stacks: []);

            await alertService.ProcessAsync(
                AlertType.DeploymentAutoUpdated,
                updatedContext,
                cancellationToken);

            return new DeploymentAutoUpdateStateWorkItem(
                deployment.Id,
                new AutoUpdateState(now, AutoUpdateStatus.UpToDate, remoteDigest, remoteDigest)
            );
        }
    }

    private async Task<(bool Success, string? Error)> TryAutoDeploy(Guid deploymentId, CancellationToken cancellationToken)
    {
        string? error = null;

        await using var scope = scopeFactory.CreateAsyncScope();
        var applyDeploymentService = scope.ServiceProvider.GetRequiredService<IApplyDeploymentService>();

        try
        {
            await foreach (var item in applyDeploymentService.ApplyAsync(deploymentId, Constants.SystemId, recreate: true, cancellationToken))
            {
                if (!string.IsNullOrEmpty(item.ErrorMessage))
                {
                    error = item.ErrorMessage;
                    break;
                }

                if (item.Error is not null)
                {
                    error = item.Error.Message;
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
}

internal sealed class DeploymentAutoUpdateStateWorkItem(
    Guid deploymentId,
    AutoUpdateState state) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        await uow.Deployments.UpdateAutoUpdateStateAsync(deploymentId, state, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}

internal sealed class DeploymentAutoUpdateFailedWorkItem(
    Guid deploymentId,
    string message) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (deployment is null) return;

        var now = DateTime.UtcNow;
        var state = new AutoUpdateState(
            now,
            AutoUpdateStatus.Failed,
            deployment.AutoUpdateState?.CurrentDigest,
            deployment.AutoUpdateState?.RemoteDigest,
            message);
        await uow.Deployments.UpdateAutoUpdateStateAsync(deploymentId, state, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}

internal sealed class SwarmServiceAutoUpdateStateWorkItem(
    Guid serviceId,
    ImageKey imageKey,
    AutoUpdateState state,
    ISwarmServiceStreamManager streamManager,
    INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
    {
        var service = await uow.SwarmServices.GetAsync(serviceId, cancellationToken);
        if (service is null
            || service.Spec.Image is not SwarmExternalImage image
            || image.RegistryId != imageKey.RegistryId
            || !Helpers.TrySplitImageTag(image.ImageTag, out var repository, out var tag)
            || !string.Equals(repository, imageKey.Repository, StringComparison.OrdinalIgnoreCase)
            || !string.Equals(tag, imageKey.Tag, StringComparison.Ordinal))
            return;

        service.SetAutoUpdateState(state);
        if (await uow.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return;

        await uow.CommitAsync(cancellationToken);
        await notificationQueue.EnqueueAsync(
            new SwarmServiceNotificationWorkItem(streamManager, service),
            cancellationToken);
    }
}

internal sealed class SwarmServiceNotificationWorkItem(
    ISwarmServiceStreamManager streamManager,
    SwarmService service) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken) =>
        streamManager.SendSwarmServiceInfo(service);
}
