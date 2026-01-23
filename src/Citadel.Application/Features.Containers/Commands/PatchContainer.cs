using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Features.Containers.Commands;

public sealed record PatchContainer(string[] ContainerIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainer>
    {
        public Validator()
            => RuleForEach(s => s.ContainerIds).ValidContainerId();
    }
}

internal sealed class PatchContainerHandler(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken ct)
    {
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError(
                "Platform resolution failed for container IDs. Platform may be disconnected."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var containers = await MarkContainersProcessingAsync(containerIds, ct);

        if (containers.Count == 0)
        {
            return Result.Failure(new NotFoundError(
                "No containers found for the provided ID(s)."));
        }

        await NotifyProcessingAsync(containers, ct);

        foreach (var platform in platforms)
        {
            var result = await PatchPlatformAsync(platform, request.Action, ct);
            if (result.IsFailure())
            {
                await RollbackProcessingAsync(containers, ct);
                return result;
            }
        }

        return Result.Success();
    }

    private async Task<List<Container>> MarkContainersProcessingAsync(Guid[] containerIds, CancellationToken ct)
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

    private async Task RollbackProcessingAsync(IEnumerable<Container> containers, CancellationToken ct)
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

    private async Task NotifyProcessingAsync(IEnumerable<Container> containers, CancellationToken ct)
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

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction action, CancellationToken ct)
    {
        var command = new PatchContainerCommand(
            Action: action,
            PlatformAddress: platform.Address,
            ContainerIds: platform.Containers.Keys);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return connector.PatchAsync(command, ct);
    }
}