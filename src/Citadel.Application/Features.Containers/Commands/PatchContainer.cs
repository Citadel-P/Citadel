using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
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

internal class PatchContainerHandler(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory) : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platformContainers))
        {
            return Result.Failure(new NotFoundError("Platform resolution failed for container IDs. Platform may be disconnected."));
        }

        var nids = platformContainers.SelectMany(s => s.Containers.Keys).ToArray();
        var successfullyUpdated = new List<Container>();

        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var containers = await uow.Containers.GetByIdAsync(nids, cancellationToken);

            if (!containers.Any())
            {
                return Result.Failure(new NotFoundError("No containers found for the provided ID (s)."));
            }

            foreach (var original in containers ?? [])
            {
                original.MarkProcessing();
                var affectedRow = await uow.Containers.UpdateProcessingAsync(
                    original.Id,
                    original.ControlState,
                    original.ControlStartedAt,
                    original.RowVersion,
                    checkRowVersion: true, 
                    cancellationToken);

                if (affectedRow != 0)
                {
                    successfullyUpdated.Add(original);
                }
            }
            await uow.CommitAsync(cancellationToken);
        }

        foreach (var container in successfullyUpdated)
        {
            await notificationQueue.EnqueueAsync(new ContainerNotificationWorkItem(container, 
                new DaemonContainerEventInfo("processing", container.DockerContainerId, null), dockerDaemonHub, containerEventBroadcaster), cancellationToken);
        }

        foreach (var platform in platformContainers)
        {
            var command = new PatchContainerCommand
            (
                Action: request.Action,
                PlatformAddress: platform.Address,
                ContainerIds: platform.Containers.Select(s => s.Key)
            );
            var result = await connectorFactory.GetConnector(platform.ConnectorType).PatchAsync(command, cancellationToken);
            if (result.IsFailure())
            {
                return result;
            }
        }

        return Result.Success();
    }
}