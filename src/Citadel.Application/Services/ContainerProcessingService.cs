using Application.Features.Containers.Commands;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

/// <summary>
/// Coordinates optimistic concurrency for container operations by marking resources as processing,
/// publishing processing notifications, and rolling back the processing state when an operation fails.
/// </summary>
internal interface IContainerProcessingService
{
    Task<ProcessedResources> MarkProcessingAsync(Guid[] containerIds, Guid controlTriggeredBy, CancellationToken ct);
    Task RollbackProcessingAsync(ProcessedResources resources, Guid controlTriggeredBy, CancellationToken ct);
    Task<Result> DeleteContainers(DeleteContainers request, Guid controlTriggeredBy, CancellationToken ct);
    Task NotifyProcessingAsync(ProcessedResources resources, CancellationToken ct);
}
internal sealed class ContainerProcessingService(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IStackStreamManager stackStreamManager,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache,
    IDeploymentStreamManager deploymentStreamManager,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory) : IContainerProcessingService
{
    public async Task<ProcessedResources> MarkProcessingAsync(Guid[] containerIds, Guid controlTriggeredBy, CancellationToken ct)
    {
        var updatedContainers = new List<Container>();
        var updatedDeployments = new List<Deployment>();
        var updatedStacks = new List<Stack>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = await uow.Containers.GetByIdAsync(containerIds, ct);

        foreach (var container in containers)
        {
            // Deployments
            if (container.DeploymentId != null)
            {
                var deployment = await uow.Deployments.GetAsync(container.DeploymentId.Value, ct);
                if (deployment != null)
                {
                    deployment.MarkProcessing(controlTriggeredBy);
                    updatedDeployments.Add(deployment);
                }
            }

            // Stacks
            if (container.StackId != null)
            {
                var stack = await uow.Stacks.GetAsync(container.StackId.Value, ct);
                if (stack != null)
                {
                    stack.MarkProcessing(controlTriggeredBy);
                    updatedStacks.Add(stack);
                }
            }

            // Containers
            container.MarkProcessing(controlTriggeredBy);

            var affected = await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);

            if (affected != 0)
            {
                updatedContainers.Add(container);
            }
        }

        await uow.CommitAsync(ct);
        return new ProcessedResources(updatedContainers, updatedDeployments, updatedStacks);
    }

    public async Task RollbackProcessingAsync(ProcessedResources resources, Guid controlTriggeredBy, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var container in resources.Containers)
        {
            container.ReleaseProcessing();

            await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);        
        }

        foreach (var deployment in resources.Deployments)
        {
            deployment.ReleaseProcessing(deployment.Status);
            await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);
        }

        foreach (var stack in resources.Stacks)
        {
            var status = stack.CurrentStackRelease?.Status ?? Domain.StackReleaseStatus.Unknown;
            stack.ReleaseProcessing(status);
            await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(resources, ct);
    }

    public async Task NotifyProcessingAsync(ProcessedResources resources, CancellationToken ct)
    {
        foreach (var container in resources.Containers)
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

        foreach (var deployment in resources.Deployments)
        {
            await notificationQueue.EnqueueAsync(
                new DeploymentNotificationWorkItem(deploymentStreamManager, deployment, "update"), ct);
        }

        foreach (var stack in resources.Stacks)
        {
            await notificationQueue.EnqueueAsync(
                new StackNotificationWorkItem(stackStreamManager, stack, "update"),ct);
        }
    }

    public async Task<Result> DeleteContainers(DeleteContainers request, Guid controlTriggeredBy, CancellationToken ct)
    {
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var processingResult = await MarkProcessingAsync(containerIds, controlTriggeredBy, ct);

        if (processingResult.Containers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        await NotifyProcessingAsync(processingResult, ct);

        foreach (var platform in platforms)
        {
            var result = await DeleteFromPlatformAsync(platform, request, ct);
            if (result.IsFailure())
            {
                await RollbackProcessingAsync(processingResult, controlTriggeredBy, ct);
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

internal sealed record ProcessedResources(List<Container> Containers, List<Deployment> Deployments, List<Stack> Stacks);