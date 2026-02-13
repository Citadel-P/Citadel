using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

/// <summary>
/// Coordinates optimistic concurrency for deployment operations by marking deployments as processing,
/// publishing processing notifications, and rolling back the processing state when an operation fails.
/// </summary>
public interface IDeploymentProcessingService
{
    Task<List<Deployment>> MarkProcessingAsync(IEnumerable<Guid> deploymentIds, Guid actorId, CancellationToken ct);
    Task RollbackProcessingAsync(IEnumerable<Deployment> deployments, CancellationToken ct);
    Task NotifyProcessingAsync(IEnumerable<Deployment> deployments, string action = "update", CancellationToken ct = default);
}

internal sealed class DeploymentProcessingService(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDeploymentStreamManager deploymentHub) : IDeploymentProcessingService
{
    public async Task<List<Deployment>> MarkProcessingAsync(IEnumerable<Guid> deploymentIds, Guid actorId, CancellationToken ct)
    {
        var successfullyUpdated = new List<Deployment>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployments = await uow.Deployments.GetAllAsync(deploymentIds, ct);
        foreach (var deployment in deployments ?? [])
        {
            deployment.MarkProcessing(actorId);

            var affectedRow = await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true,
                actorId,
                ct);

            if (affectedRow != 0)
            {
                successfullyUpdated.Add(deployment);
            }
        }

        await uow.CommitAsync(ct);
        return successfullyUpdated;
    }

    public async Task RollbackProcessingAsync(IEnumerable<Deployment> deployments, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var deployment in deployments)
        {
            deployment.ReleaseProcessing(deployment.Status);

            await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true,
                deployment.ControlTriggeredBy != null ? deployment.ControlTriggeredBy.Value : Constants.SystemId,
                ct);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(deployments, ct: ct);
    }

    public async Task NotifyProcessingAsync(IEnumerable<Deployment> deployments, string action = "update", CancellationToken ct = default)
    {
        foreach (var deployment in deployments)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentHub, deployment, action), ct);
        }
    }
}
