using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

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
    IDeploymentStreamManager deploymentHub,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeploymentProcessingService> logger) : IDeploymentProcessingService
{
    public async Task<List<Deployment>> MarkProcessingAsync(IEnumerable<Guid> deploymentIds, Guid actorId, CancellationToken ct)
    {
        var successfullyUpdated = new List<Deployment>();
        var requestedIds = deploymentIds.Distinct().Order().ToArray();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployments = (await uow.Deployments.GetAllAsync(requestedIds, ct) ?? [])
            .OrderBy(deployment => deployment.Id)
            .ToArray();

        if (deployments.Length != requestedIds.Length ||
            deployments.Any(deployment => deployment.ControlState == ResourceControlState.Processing))
        {
            return [];
        }

        foreach (var deployment in deployments)
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

            if (affectedRow == 0)
                return [];

            successfullyUpdated.Add(deployment);
        }

        await uow.CommitAsync(ct);
        return successfullyUpdated;
    }

    public async Task RollbackProcessingAsync(IEnumerable<Deployment> deployments, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var released = new List<Deployment>();

        foreach (var deployment in deployments)
        {
            deployment.ReleaseProcessing(deployment.Status);

            var affected = await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);
            if (affected != 0)
                released.Add(deployment);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(released, ct: ct);
    }

    public async Task NotifyProcessingAsync(IEnumerable<Deployment> deployments, string action = "update", CancellationToken ct = default)
    {
        using var notificationCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            ct);
        notificationCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        try
        {
            foreach (var deployment in deployments)
            {
                await notificationQueue.EnqueueAsync(
                    new DeploymentNotificationWorkItem(deploymentHub, deployment, action),
                    notificationCancellation.Token);
            }
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to enqueue deployment processing notifications");
        }
    }
}
