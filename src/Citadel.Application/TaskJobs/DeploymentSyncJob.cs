using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

/// <summary>
/// Synchronizes deployment state.
/// </summary>
internal class DeploymentSyncJob(
    IDbWorkQueue dbWorkQueue,
    INotificationQueue notifQueue,
    IPlatformContainerCache platformContainerCache,
    IDeploymentStreamManager deploymentStreamManager,
    ILogger<DeploymentSyncJob> logger) : BackgroundService
{
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{JobName} started. Running every {H} hours.", nameof(DeploymentSyncJob), SyncInterval.TotalHours);

        await Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

    }

    private async Task RunPeriodicSync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                if (platformContainerCache.TryGetCacheEntries(out var platforms, out _))
                {
                    foreach (var platform in platforms)
                    {
                        await dbWorkQueue.EnqueueAsync(new DeploymentSyncWorkItem(deploymentStreamManager, notifQueue, platform.Id, true), ct);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "periodic deployment sync failed");
            }

            await Task.Delay(SyncInterval, ct);
        }
    }
}

internal sealed class DeploymentSyncWorkItem(
    IDeploymentStreamManager deploymentStreamManager,
    INotificationQueue notificationQueue,
    Guid platformId,
    bool platformIsOnline
) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var updated = new List<Deployment>();
        var deployments = await uow.Deployments.GetByPlatformIdAsync(platformId, ct);

        if (!platformIsOnline)
        {
            foreach (var deployment in deployments)
            {
                if (deployment.Status != DeploymentStatus.Created)
                {
                    deployment.PartialUpdate(status: DeploymentStatus.Degraded);
                    updated.Add(deployment);
                }
            }

            await uow.Deployments.UpdateStatusAsync(updated.Select(s => s.Id), DeploymentStatus.Degraded, ct);
        }
        else
        {
            var containers = await uow.Containers.GetByDeploymentIdsAsync(deployments.Select(d => d.Id), ct);
            foreach (var container in containers) 
            {
                var status = Deployment.ToDeploymentStatus(container.State);
                var deployment = deployments.FirstOrDefault(d => d.Id == container.DeploymentId);
                if (deployment != null) 
                {
                    if (deployment.Status != status)
                    {
                        deployment.PartialUpdate(status: status);
                        await uow.Deployments.UpdateStatusAsync([deployment.Id], status, ct);
                        updated.Add(deployment);
                    }
                }
            }
        }

        await uow.CommitAsync(ct);

        foreach (var dep in updated)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentStreamManager, dep), ct);
        }

    }
}
