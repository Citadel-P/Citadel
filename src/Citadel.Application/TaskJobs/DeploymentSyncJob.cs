using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

/// <summary>
/// Synchronizes deployment state from platform health and container state.
/// </summary>
internal sealed class DeploymentSyncJob(
    IDbWorkQueue dbWorkQueue,
    INotificationQueue notifQueue,
    IPlatformContainerCache platformContainerCache,
    IDeploymentStreamManager deploymentStreamManager,
    IPlatformHealthBroadCaster platformHealthBroadCaster,
    ILogger<DeploymentSyncJob> logger) : BackgroundService
{
    private readonly ChannelReader<PlatformHealth> platformHealthReader = platformHealthBroadCaster.AddSubscriber();
    private static readonly TimeSpan SyncInterval = TimeSpan.FromHours(6);

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        logger.LogInformation("{JobName} started. Running every {H} hours.", nameof(DeploymentSyncJob), SyncInterval.TotalHours);

        var eventDrivenTask = RunEventDrivenSync(cancellationToken);
        var periodicTask = Helpers.DelayWithJitterFor(RunPeriodicSync, cancellationToken: cancellationToken);

        await Task.WhenAll(eventDrivenTask, periodicTask);
    }

    private async Task RunEventDrivenSync(CancellationToken ct)
    {
        await foreach (var platformEvent in platformHealthReader.ReadAllAsync(ct))
        {
            try
            {
                if (platformEvent.IsOnLine && !platformEvent.IsValidated)
                    continue;

                await ScheduleDeploymentSync(platformEvent.Id, platformEvent.IsOnLine, ct);
            }
            catch (Exception ex)
            {
                logger.LogError(
                    ex,
                    "Unhandled error while scheduling deployment sync for platform {PlatformId}",
                    platformEvent.Id);
            }
        }
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
                        await ScheduleDeploymentSync(platform.Id, true, ct);
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

    private ValueTask ScheduleDeploymentSync(Guid platformId, bool isOnline, CancellationToken ct)
        => dbWorkQueue.EnqueueAsync(new DeploymentSyncWorkItem(deploymentStreamManager, notifQueue, platformId, isOnline), ct);
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
        var deployments = (await uow.Deployments.GetByPlatformIdAsync(platformId, ct))
            .Where(static deployment => deployment.Platform?.PlatformDescriptor.Type == PlatformType.Docker)
            .ToArray();

        if (!platformIsOnline)
        {
            foreach (var deployment in deployments)
            {
                if (deployment.Status is DeploymentStatus.Created or DeploymentStatus.Degraded)
                    continue;

                deployment.PartialUpdate(status: DeploymentStatus.Degraded);
                updated.Add(deployment);
            }

            if (updated.Count > 0)
            {
                await uow.Deployments.UpdateStatusAsync(updated.Select(s => s.Id), DeploymentStatus.Degraded, ct);
            }
        }
        else
        {
            var containers = await uow.Containers.GetByDeploymentIdsAsync(deployments.Select(d => d.Id), ct);
            var deploymentsById = deployments.ToDictionary(d => d.Id);

            foreach (var container in containers)
            {
                if (container.DeploymentId is null || !deploymentsById.TryGetValue(container.DeploymentId.Value, out var deployment))
                    continue;

                var status = Deployment.ToDeploymentStatus(container.State);
                if (deployment.Status == status)
                    continue;

                deployment.PartialUpdate(status: status);
                await uow.Deployments.UpdateStatusAsync([deployment.Id], status, ct);
                updated.Add(deployment);
            }
        }

        await uow.CommitAsync(ct);

        foreach (var dep in updated)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentStreamManager, dep), ct);
        }
    }
}
