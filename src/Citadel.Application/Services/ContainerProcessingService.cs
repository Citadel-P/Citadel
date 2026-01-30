using Application.Features.Containers.Commands;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

/// <summary>
/// Coordinates optimistic concurrency for container operations by marking containers as processing,
/// publishing processing notifications, and rolling back the processing state when an operation fails.
/// </summary>
internal interface IContainerProcessingService
{
    Task<List<Container>> MarkProcessingAsync(Guid[] containerIds, CancellationToken ct);
    Task RollbackProcessingAsync(IEnumerable<Container> containers, CancellationToken ct);
    Task NotifyProcessingAsync(IEnumerable<Container> containers, CancellationToken ct);
    Task<Result> DeleteContainers(DeleteContainers request, CancellationToken ct);
}

internal sealed class ContainerProcessingService(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory) : IContainerProcessingService
{
    public async Task<List<Container>> MarkProcessingAsync(Guid[] containerIds, CancellationToken ct)
    {
        var updated = new List<Container>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await uow.Containers.GetByIdAsync(containerIds, ct);

        foreach (var container in containers)
        {
            container.MarkProcessing();

            var affected = await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion,
                checkRowVersion: true,
                ct);

            if (affected != 0)
            {
                updated.Add(container);
            }
        }

        await uow.CommitAsync(ct);
        return updated;
    }

    public async Task RollbackProcessingAsync(IEnumerable<Container> containers, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var container in containers)
        {
            container.ReleaseProcessing();

            await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion,
                checkRowVersion: true,
                ct);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(containers, ct);
    }

    public async Task NotifyProcessingAsync(IEnumerable<Container> containers, CancellationToken ct)
    {
        foreach (var container in containers)
        {
            await notificationQueue.EnqueueAsync(
                new ContainerNotificationWorkItem(
                    container,
                    new DaemonContainerEventInfo(
                        "processing",
                        container.DockerContainerId,
                        null),
                    dockerDaemonHub,
                    containerEventBroadcaster),
                ct);
        }
    }

    public async Task<Result> DeleteContainers(DeleteContainers request, CancellationToken ct)
    {
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var containers = await MarkProcessingAsync(containerIds, ct);

        if (containers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        await NotifyProcessingAsync(containers, ct);

        foreach (var platform in platforms)
        {
            var result = await DeleteFromPlatformAsync(platform, request, ct);
            if (result.IsFailure())
            {
                await RollbackProcessingAsync(containers, ct);
                return result;
            }
        }

        return Result.Success();
    }

    private async Task<Result> DeleteFromPlatformAsync(PlatformCacheEntry platform, DeleteContainers request, CancellationToken ct)
    {
        var command = new DeleteContainerCommand(
            ContainerIds: platform.Containers.Keys,
            PlatformAddress: platform.Address,
            Volume: request.V,
            Force: request.Force,
            Link: request.Link);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        await connector.DeleteAsync(command, ct);
        return Result.Success();
    }
}
