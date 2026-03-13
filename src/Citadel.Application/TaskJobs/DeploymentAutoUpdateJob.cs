using Application.Services;
using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class DeploymentAutoUpdateJob(
    IDbWorkQueue dbWorkQueue,
    IAlertService alertService,
    IImageScanScheduler imageScanScheduler,
    ImageDigestCache imageDigestCache,
    IServiceScopeFactory scopeFactory,
    IDelayWithJitterService delayWithJitterService,
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
                var deploymentChecks = await imageScanScheduler.LoadDeploymentChecksAsync(cancellationToken);
                foreach (var deploymentCheck in deploymentChecks)
                {
                    try
                    {
                        var result = await CheckDeploymentAsync(deploymentCheck, cancellationToken);
                        if (result is not null)
                        {
                            await dbWorkQueue.EnqueueAsync(result, cancellationToken);
                        }
                    }
                    catch (Exception ex)
                    {
                        logger.LogError(ex, "Auto-update failed for deployment {DeploymentId}", deploymentCheck.Deployment.Id);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error occurred while running deployment auto-update job.");
            }

            await Task.Delay(TimeSpan.FromHours(CheckIntervalInHours), cancellationToken);
        }
    }

    private async Task<IDbWorkItem?> CheckDeploymentAsync(
        DeploymentImageCheck deploymentCheck,
        CancellationToken cancellationToken)
    {
        var deployment = deploymentCheck.Deployment;
        var deployedImage = deploymentCheck.DeployedImage;

        if (!imageDigestCache.TryGet(deploymentCheck.Key, out var digestEntry))
            return null;

        var remoteDigest = digestEntry.Digest;
        var currentDigest = deployedImage.ResolvedDigest;
        var matchedTag = remoteDigest.Equals(currentDigest, StringComparison.OrdinalIgnoreCase);

        var now = DateTime.UtcNow;

        if (matchedTag)
        {
            return new DeploymentAutoUpdateStateWorkItem(
                deployment.Id,
                new AutoUpdateState(now, AutoUpdateStatus.UpToDate, remoteDigest, remoteDigest)
            );
        }

        if (deployment.Spec?.UpdateBehavior == UpdateBehavior.Notify)
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
                new AutoUpdateState(now, AutoUpdateStatus.UpdateAvailable, currentDigest, remoteDigest)
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
        var deployment = await uow.Deployments.GetAsync(deploymentId, cancellationToken);
        if (deployment is null) return;

        deployment.SetAutoUpdateState(state);
        await uow.Deployments.UpdateAsync(deployment, cancellationToken);
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
        deployment.SetAutoUpdateState(
            new AutoUpdateState(
                now,
                AutoUpdateStatus.Failed,
                deployment.AutoUpdateState?.CurrentDigest,
                deployment.AutoUpdateState?.RemoteDigest,
                message));

        await uow.Deployments.UpdateAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }
}