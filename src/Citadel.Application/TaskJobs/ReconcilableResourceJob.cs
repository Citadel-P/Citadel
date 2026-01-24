using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal class ReconcilableResourceJob(
    IDbWorkQueue dbWorkQueue,
    INotificationQueue notifQueue,
    IServiceScopeFactory scopeFactory,
    IDeploymentStreamManager deploymentHub,
    IDockerDaemonStreamManager dockerDaemonHub,
    IContainerEventBroadcaster containerEventBroadcaster,
    ILogger<ReconcilableResourceJob> logger) : BackgroundService
{
    private static readonly TimeSpan SyncInterval = TimeSpan.FromMinutes(2);
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await Helpers.DelayWithJitterFor(RunPeriodicJanitor, maxJitter: TimeSpan.FromSeconds(20), cancellationToken: stoppingToken);
    }

    private async Task RunPeriodicJanitor(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            try
            {
                await using (var scope = scopeFactory.CreateAsyncScope())
                {
                    var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

                    // Deployments
                    var stuckDeployments = await uow.Deployments.GetStuckDeploymentsAsync(cancellationToken: cancellationToken);
                    if (stuckDeployments.Any())
                    {
                        var workItem = new StuckDeploymentsSyncWorkItem(deploymentHub, notifQueue, stuckDeployments);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Containers
                    var stuckContainers = await uow.Containers.GetStuckContainersAsync(cancellationToken: cancellationToken);
                    if (stuckContainers.Any())
                    {
                        var workItem = new StuckContainersSyncWorkItem(deploymentHub, notifQueue, dockerDaemonHub, containerEventBroadcaster, stuckContainers);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }

                    // Images
                    var stuckImages = await uow.Images.GetStuckImagesAsync(cancellationToken: cancellationToken);
                    if (stuckImages.Any())
                    {
                        var workItem = new StuckImagesSyncWorkItem(notifQueue, dockerDaemonHub, stuckImages);
                        await dbWorkQueue.EnqueueAsync(workItem, cancellationToken);
                    }
                }
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error during periodic {JobName} synchronization.", nameof(ReconcilableResourceJob));
            }

            await Task.Delay(SyncInterval, cancellationToken);
        }
    }

    internal sealed class StuckDeploymentsSyncWorkItem(
        IDeploymentStreamManager deploymentHub,
        INotificationQueue notificationQueue,
        IEnumerable<Deployment> deployments)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Deployment>();

            foreach (var deployment in deployments)
            {
                deployment.ReleaseProcessing(DeploymentStatus.Unknown);
                var row = await uow.Deployments.UpdateProcessingAsync(
                                    id: deployment.Id,
                                    status: deployment.Status,
                                    state: deployment.ControlState,
                                    startedAt: null,
                                    rowVersion: deployment.RowVersion,
                                    checkRowVersion: true,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(deployment);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var deployment in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentHub, deployment), cancellationToken);
            }
        }
    }

    internal sealed class StuckContainersSyncWorkItem(
        IDeploymentStreamManager deploymentHub,
        INotificationQueue notificationQueue,
        IDockerDaemonStreamManager dockerDaemonHub,
        IContainerEventBroadcaster containerEventBroadcaster,
        IEnumerable<Container> containers)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Container>();

            foreach (var container in containers)
            {
                container.ReleaseProcessing();
                var row = await uow.Containers.UpdateProcessingAsync(
                                    id: container.Id,
                                    state: container.ControlState,
                                    startedAt: null,
                                    rowVersion: container.RowVersion,
                                    checkRowVersion: true,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(container);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var container in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new ContainerNotificationWorkItem(container,
                new DaemonContainerEventInfo("processing", container.DockerContainerId, null), dockerDaemonHub, containerEventBroadcaster), cancellationToken);
            }
        }
    }

    internal sealed class StuckImagesSyncWorkItem(
        INotificationQueue notificationQueue,
        IDockerDaemonStreamManager dockerDaemonHub,
        IEnumerable<Image> images)
        : IDbWorkItem
    {
        public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken cancellationToken)
        {
            var successfullyUpdated = new List<Image>();

            foreach (var container in images)
            {
                container.ReleaseProcessing();
                var row = await uow.Images.UpdateProcessingAsync(
                                    id: container.Id,
                                    state: container.ControlState,
                                    startedAt: null,
                                    rowVersion: container.RowVersion,
                                    checkRowVersion: true,
                                    cancellationToken);
                if (row > 0)
                {
                    successfullyUpdated.Add(container);
                }
            }

            await uow.CommitAsync(cancellationToken);

            foreach (var image in successfullyUpdated)
            {
                await notificationQueue.EnqueueAsync(new ImageNotificationWorkItem(dockerDaemonHub, image, "update"), cancellationToken);
            }
        }
    }
}
