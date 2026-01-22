using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
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

                    var stuckDeployments = await uow.Deployments.GetStuckDeploymentsAsync(cancellationToken: cancellationToken);
                    if (stuckDeployments.Any())
                    {
                        var workItem = new StuckDeploymentsSyncWorkItem(deploymentHub, notifQueue, stuckDeployments);
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
                deployment.PartialUpdate(
                    status: DeploymentStatus.Unknown,
                    resourceControlState: ResourceControlState.Idle
                    );
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
}
