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
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

using Application.Features.Deployments.Notifications;
using Domain;
using Domain.Entities.Activities;

namespace Application.Services;

/// <summary>
/// Coordinates optimistic concurrency for container operations by marking resources as processing,
/// publishing processing notifications, and rolling back the processing state when an operation fails.
/// </summary>
internal interface IContainerProcessingService
{
    Task<ProcessedResources> MarkProcessingAsync(
        Guid[] containerIds,
        Guid controlTriggeredBy,
        CancellationToken ct,
        bool claimParentResources = true);
    Task CompleteProcessingAsync(ProcessedResources resources, IReadOnlyCollection<PlatformCacheEntry> platforms, Guid controlTriggeredBy);
    Task RollbackProcessingAsync(ProcessedResources resources, Guid controlTriggeredBy, CancellationToken ct);
    Task<Result> DeleteContainers(
        DeleteContainers request,
        Guid controlTriggeredBy,
        CancellationToken ct,
        bool claimParentResources = true);
    Task NotifyProcessingAsync(ProcessedResources resources, CancellationToken ct);
}
internal sealed class ContainerProcessingService(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IStackStreamManager stackStreamManager,
    IDockerDaemonStreamManager dockerDaemonHub,
    IActivityStreamManager activityStreamManager,
    IPlatformContainerCache platformContainerCache,
    IDeploymentStreamManager deploymentStreamManager,
    IContainerEventBroadcaster containerEventBroadcaster,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IDbWorkQueue dbWorkQueue,
    IHostApplicationLifetime applicationLifetime,
    ILogger<ContainerProcessingService> logger) : IContainerProcessingService
{
    private static readonly TimeSpan CommandCompletionTimeout = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan OperationTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);
    private static readonly TimeSpan NotificationTimeout = TimeSpan.FromSeconds(5);

    public async Task<ProcessedResources> MarkProcessingAsync(
        Guid[] containerIds,
        Guid controlTriggeredBy,
        CancellationToken ct,
        bool claimParentResources = true)
    {
        var requestedIds = containerIds.Distinct().Order().ToArray();
        var updatedContainers = new List<Container>();
        var candidateDeployments = new Dictionary<Guid, Deployment>();
        var candidateStacks = new Dictionary<Guid, Stack>();
        var updatedDeployments = new List<Deployment>();
        var updatedStacks = new List<Stack>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var containers = (await uow.Containers.GetByIdAsync(requestedIds, ct))
            .OrderBy(container => container.Id)
            .ToArray();

        if (containers.Length != requestedIds.Length ||
            containers.Any(container => container.ControlState == ResourceControlState.Processing))
        {
            return ProcessedResources.Conflict;
        }

        foreach (var container in containers)
        {
            if (claimParentResources && container.DeploymentId != null)
            {
                var deployment = await uow.Deployments.GetAsync(container.DeploymentId.Value, ct);
                if (deployment is null || deployment.ControlState == ResourceControlState.Processing)
                    return ProcessedResources.Conflict;

                candidateDeployments.TryAdd(deployment.Id, deployment);
            }

            if (claimParentResources && container.StackId != null)
            {
                var stack = await uow.Stacks.GetAsync(container.StackId.Value, ct);
                if (stack?.CurrentStackRelease is null || stack.ControlState == ResourceControlState.Processing)
                    return ProcessedResources.Conflict;

                candidateStacks.TryAdd(stack.Id, stack);
            }
        }

        foreach (var container in containers)
        {
            container.MarkProcessing(controlTriggeredBy);

            var affected = await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);

            if (affected == 0)
                return ProcessedResources.Conflict;

            updatedContainers.Add(container);
        }

        foreach (var deployment in candidateDeployments.Values.OrderBy(deployment => deployment.Id))
        {
            deployment.MarkProcessing(controlTriggeredBy);
            var affected = await uow.Deployments.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                deployment.ControlState,
                deployment.ControlStartedAt,
                deployment.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);

            if (affected == 0)
                return ProcessedResources.Conflict;

            updatedDeployments.Add(deployment);
        }

        foreach (var stack in candidateStacks.Values.OrderBy(stack => stack.Id))
        {
            if (!stack.MarkProcessing(controlTriggeredBy))
                return ProcessedResources.Conflict;

            var status = stack.CurrentStackRelease?.Status ?? Domain.StackReleaseStatus.Unknown;
            var affected = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion,
                checkRowVersion: true,
                controlTriggeredBy,
                ct);

            if (!affected)
                return ProcessedResources.Conflict;

            updatedStacks.Add(stack);
        }

        await uow.CommitAsync(ct);
        return new ProcessedResources(updatedContainers, updatedDeployments, updatedStacks);
    }

    public async Task CompleteProcessingAsync(
        ProcessedResources resources,
        IReadOnlyCollection<PlatformCacheEntry> platforms,
        Guid controlTriggeredBy)
    {
        using var completionCts = CancellationTokenSource.CreateLinkedTokenSource(applicationLifetime.ApplicationStopping);
        completionCts.CancelAfter(CommandCompletionTimeout);
        var completionToken = completionCts.Token;

        try
        {
            var runtimeStates = new Dictionary<string, ContainerStateStatus>(StringComparer.OrdinalIgnoreCase);

            foreach (var platform in platforms)
            {
                var connector = connectorFactory.GetConnector(platform.ConnectorType);
                foreach (var containerId in platform.Containers.Keys)
                {
                    var result = await connector.InspectAsync(
                        new InspectContainerCommand(platform.Address, containerId),
                        completionToken);

                    if (!result.IsSuccess(out var inspection) || inspection.State is null)
                        return;

                    runtimeStates[containerId] = inspection.State.Status;
                }
            }

            if (runtimeStates.Count == 0)
                return;

            await dbWorkQueue.EnqueueAndWaitAsync(
                new CompleteContainerCommandWorkItem(
                    resources,
                    runtimeStates,
                    controlTriggeredBy,
                    notificationQueue,
                    dockerDaemonHub,
                    containerEventBroadcaster,
                    deploymentStreamManager,
                    stackStreamManager,
                    logger),
                completionToken);
        }
        catch (OperationCanceledException) when (!applicationLifetime.ApplicationStopping.IsCancellationRequested)
        {
            // The daemon event remains the normal fallback if the bounded authoritative refresh times out.
        }
    }

    public async Task RollbackProcessingAsync(ProcessedResources resources, Guid controlTriggeredBy, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var releasedContainers = new List<Container>();
        var releasedDeployments = new List<Deployment>();
        var releasedStacks = new List<Stack>();

        foreach (var container in resources.Containers)
        {
            container.ReleaseProcessing();

            var affected = await uow.Containers.UpdateProcessingAsync(
                container.Id,
                container.ControlState,
                container.ControlStartedAt,
                container.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);
            if (affected != 0)
                releasedContainers.Add(container);
        }

        foreach (var deployment in resources.Deployments)
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
                releasedDeployments.Add(deployment);
        }

        foreach (var stack in resources.Stacks)
        {
            var status = stack.CurrentStackRelease?.Status ?? Domain.StackReleaseStatus.Unknown;
            stack.ReleaseProcessing(status);
            var affected = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                status,
                stack.ControlState,
                stack.ControlStartedAt,
                stack.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);
            if (affected)
                releasedStacks.Add(stack);
        }

        await uow.CommitAsync(ct);
        await NotifyProcessingAsync(
            new ProcessedResources(releasedContainers, releasedDeployments, releasedStacks),
            ct);
    }

    public async Task NotifyProcessingAsync(ProcessedResources resources, CancellationToken ct)
    {
        using var notificationCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping,
            ct);
        notificationCancellation.CancelAfter(NotificationTimeout);

        try
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
                    notificationCancellation.Token);
            }

            foreach (var deployment in resources.Deployments)
            {
                await notificationQueue.EnqueueAsync(
                    new DeploymentNotificationWorkItem(deploymentStreamManager, deployment, "update"),
                    notificationCancellation.Token);
            }

            foreach (var stack in resources.Stacks)
            {
                await notificationQueue.EnqueueAsync(
                    new StackNotificationWorkItem(stackStreamManager, stack, "update"),
                    notificationCancellation.Token);
            }
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to enqueue container processing notifications");
        }
    }

    public async Task<Result> DeleteContainers(
        DeleteContainers request,
        Guid controlTriggeredBy,
        CancellationToken ct,
        bool claimParentResources = true)
    {
        var hasCachedContainers = platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms);
        platforms ??= [];

        var cachedContainerIds = platforms
            .SelectMany(platform => platform.Containers.Values)
            .ToArray();

        var staleContainers = await GetStalePersistedContainersAsync(request.ContainerIds, cachedContainerIds, ct);

        if (!hasCachedContainers && staleContainers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var processingResult = containerIds.Length > 0
            ? await MarkProcessingAsync(containerIds, controlTriggeredBy, ct, claimParentResources)
            : new ProcessedResources([], [], []);

        if (processingResult.HasConflict)
        {
            return Result.Failure(new ConflictError(
                "One or more containers, deployments, or stacks are already processing another operation."));
        }

        if (processingResult.Containers.Count == 0 && staleContainers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(OperationTimeout);
        var completionToken = completionCancellation.Token;
        try
        {
            await NotifyProcessingAsync(processingResult, completionToken);

            foreach (var platform in platforms)
            {
                var result = await DeleteFromPlatformAsync(platform, request, completionToken);
                if (result.IsFailure())
                {
                    await TryRollbackProcessingAsync(processingResult, controlTriggeredBy);
                    return result;
                }
            }

            if (staleContainers.Count > 0)
            {
                await DeleteStalePersistedContainersAsync(staleContainers, completionToken);
            }

            return Result.Success();
        }
        catch
        {
            await TryRollbackProcessingAsync(processingResult, controlTriggeredBy);
            throw;
        }
    }

    private async Task TryRollbackProcessingAsync(
        ProcessedResources resources,
        Guid controlTriggeredBy)
    {
        if (resources.Containers.Count == 0 &&
            resources.Deployments.Count == 0 &&
            resources.Stacks.Count == 0)
        {
            return;
        }

        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await RollbackProcessingAsync(
                resources,
                controlTriggeredBy,
                rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back container deletion claims");
        }
    }

    private async Task<List<Container>> GetStalePersistedContainersAsync(
        string[] requestedContainerIds,
        Guid[] cachedContainerIds,
        CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var staleContainers = await uow.Containers.GetStaleByDockerIdsAsync(requestedContainerIds, cachedContainerIds, ct);

        return staleContainers.ToList();
    }

    private async Task DeleteStalePersistedContainersAsync(IReadOnlyCollection<Container> containers, CancellationToken ct)
    {
        if (containers.Count == 0)
            return;

        var deletedContainers = new List<Container>();
        var updatedImages = new List<Image>();
        var updatedDeployments = new List<Deployment>();
        var updatedStacks = new List<Stack>();
        var activities = new List<ActivityEvent>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var container in containers)
        {
            var image = await UpdateImageContainerCountAsync(uow, container, ct);
            if (image is not null)
            {
                updatedImages.Add(image);
            }

            if (container.DeploymentId is not null)
            {
                var (deployment, activity) = await ContainerDestroyedWorkItem.UpdateDeploymentStatus(
                    uow,
                    container.DeploymentId.Value,
                    DeploymentStatus.Degraded,
                    container.DockerContainerId,
                    ct);

                if (deployment is not null)
                {
                    updatedDeployments.Add(deployment);
                }

                if (activity is not null)
                {
                    activities.Add(activity);
                }
            }

            if (container.StackId is not null)
            {
                var (stack, activity) = await ContainerDestroyedWorkItem.UpdateStackStatus(
                    uow,
                    container.StackId.Value,
                    [new StackContainerState(container.DockerContainerId, ContainerStateStatus.Offline)],
                    ContainerStateStatus.Offline,
                    container.DockerContainerId,
                    StackReleaseStatus.Degraded,
                    ct);

                if (stack is not null)
                {
                    updatedStacks.Add(stack);
                }

                if (activity is not null)
                {
                    activities.Add(activity);
                }
            }

            deletedContainers.Add(container);
        }

        await uow.Containers.DeleteAsync(deletedContainers.Select(container => container.Id), ct);
        await uow.CommitAsync(ct);

        foreach (var container in deletedContainers)
        {
            platformContainerCache.TryRemoveContainer(container.PlatformId, container.DockerContainerId);

            await notificationQueue.EnqueueAsync(
                new ContainerNotificationWorkItem(
                    container,
                    new DaemonContainerEventInfo("destroy", container.DockerContainerId, null),
                    dockerDaemonHub,
                    containerEventBroadcaster),
                ct);
        }

        foreach (var image in updatedImages)
        {
            await notificationQueue.EnqueueAsync(new ImageNotificationWorkItem(dockerDaemonHub, image, "update"), ct);
        }

        foreach (var deployment in updatedDeployments)
        {
            await notificationQueue.EnqueueAsync(new DeploymentNotificationWorkItem(deploymentStreamManager, deployment), ct);
        }

        foreach (var stack in updatedStacks)
        {
            await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackStreamManager, stack), ct);
        }

        foreach (var activity in activities)
        {
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(uow, ct)), ct);
        }
    }

    private static async Task<Image?> UpdateImageContainerCountAsync(IUnitOfWork uow, Container container, CancellationToken ct)
    {
        if (string.IsNullOrEmpty(container.DockerImageId))
            return null;

        var image = await uow.Images.GetByDockerImageIdAsync(container.DockerImageId, container.PlatformId, ct);
        if (image is null)
            return null;

        image.PartialUpdate(containers: Math.Max(0, image.Containers - 1));
        await uow.Images.AddOrUpdateAsync(image, ct);

        return image;
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
        return await connector.DeleteAsync(command, ct);
    }
}

internal sealed class CompleteContainerCommandWorkItem(
    ProcessedResources resources,
    IReadOnlyDictionary<string, ContainerStateStatus> runtimeStates,
    Guid controlTriggeredBy,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IContainerEventBroadcaster containerEventBroadcaster,
    IDeploymentStreamManager deploymentStreamManager,
    IStackStreamManager stackStreamManager,
    ILogger<ContainerProcessingService> logger) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var claimedContainers = resources.Containers.ToDictionary(container => container.Id);
        var ownedDeploymentIds = new HashSet<Guid>();
        var ownedStackIds = new HashSet<Guid>();
        var updatedContainers = new List<Container>();
        var updatedDeployments = new List<Deployment>();
        var updatedStacks = new List<Stack>();

        foreach (var claimed in resources.Deployments)
        {
            var deployment = await uow.Deployments.GetAsync(claimed.Id, ct);
            if (deployment is not null &&
                IsOwnedClaim(
                    deployment.ControlState,
                    deployment.ControlTriggeredBy,
                    deployment.RowVersion,
                    claimed.RowVersion + 1))
            {
                ownedDeploymentIds.Add(deployment.Id);
            }
        }

        foreach (var claimed in resources.Stacks)
        {
            var stack = await uow.Stacks.GetAsync(claimed.Id, ct);
            if (stack is not null &&
                IsOwnedClaim(
                    stack.ControlState,
                    stack.ControlTriggeredBy,
                    stack.RowVersion,
                    claimed.RowVersion + 1))
            {
                ownedStackIds.Add(stack.Id);
            }
        }

        foreach (var (dockerContainerId, state) in runtimeStates)
        {
            var container = await uow.Containers.GetContainerInfoAsync(dockerContainerId, ct);
            if (container is null)
                continue;

            var ownsContainerClaim = claimedContainers.TryGetValue(container.Id, out var claimed) &&
                                     IsOwnedClaim(
                                         container.ControlState,
                                         container.ControlTriggeredBy,
                                         container.RowVersion,
                                         claimed.RowVersion + 1);
            var ownsParentClaim = container.DeploymentId is { } deploymentId &&
                                  ownedDeploymentIds.Contains(deploymentId) ||
                                  container.StackId is { } stackId &&
                                  ownedStackIds.Contains(stackId);

            // A daemon event may have completed this operation after the inspection was
            // captured but before this work item ran. Never overwrite that newer event.
            if (!ownsContainerClaim && !ownsParentClaim)
                continue;

            if (ownsContainerClaim)
            {
                container.ReleaseProcessing();
                var released = await uow.Containers.UpdateProcessingAsync(
                    container.Id,
                    container.ControlState,
                    container.ControlStartedAt,
                    claimed!.RowVersion + 1,
                    checkRowVersion: true,
                    controlTriggeredBy: null,
                    ct);
                if (released == 0)
                    continue;
            }

            container.PartialUpdate(state: state);
            await uow.Containers.UpdateAsync(container, ct);
            updatedContainers.Add(container);
        }

        if (resources.Deployments.Count > 0)
        {
            var deploymentContainers = (await uow.Containers.GetByDeploymentIdsAsync(
                    resources.Deployments.Select(deployment => deployment.Id),
                    ct))
                .Where(container => container.DeploymentId.HasValue)
                .ToDictionary(container => container.DeploymentId!.Value);

            foreach (var claimed in resources.Deployments)
            {
                var deployment = await uow.Deployments.GetAsync(claimed.Id, ct);
                if (deployment is null ||
                    !deploymentContainers.TryGetValue(deployment.Id, out var container) ||
                    !IsOwnedClaim(deployment.ControlState, deployment.ControlTriggeredBy, deployment.RowVersion, claimed.RowVersion + 1))
                {
                    continue;
                }

                deployment.ReleaseProcessing(Deployment.ToDeploymentStatus(container.State));
                var affected = await uow.Deployments.UpdateProcessingAsync(
                    deployment.Id,
                    deployment.Status,
                    deployment.ControlState,
                    deployment.ControlStartedAt,
                    claimed.RowVersion + 1,
                    checkRowVersion: true,
                    controlTriggeredBy: null,
                    ct);

                if (affected != 0)
                    updatedDeployments.Add(deployment);
            }
        }

        foreach (var claimed in resources.Stacks)
        {
            var stack = await uow.Stacks.GetAsync(claimed.Id, ct);
            if (stack?.CurrentStackRelease is null ||
                !IsOwnedClaim(stack.ControlState, stack.ControlTriggeredBy, stack.RowVersion, claimed.RowVersion + 1))
            {
                continue;
            }

            var containers = (await uow.Stacks.GetContainersAsync(stack.Id, ct)).ToArray();
            if (containers.Length == 0)
                continue;

            var status = Stack.ToStackStatus(containers.Select(container => container.State));
            stack.ReleaseProcessing(status);
            var affected = await uow.Stacks.UpdateProcessingAsync(
                stack.Id,
                status,
                stack.ControlState,
                stack.ControlStartedAt,
                claimed.RowVersion + 1,
                checkRowVersion: true,
                controlTriggeredBy: null,
                ct);

            if (affected)
                updatedStacks.Add(stack);
        }

        await uow.CommitAsync(ct);

        foreach (var container in updatedContainers)
        {
            await TryNotifyAsync(
                new ContainerNotificationWorkItem(
                    container,
                    new DaemonContainerEventInfo("reconcile", container.DockerContainerId, null),
                    dockerDaemonHub,
                    containerEventBroadcaster),
                ct);
        }

        foreach (var deployment in updatedDeployments)
        {
            await TryNotifyAsync(
                new DeploymentNotificationWorkItem(deploymentStreamManager, deployment),
                ct);
        }

        foreach (var stack in updatedStacks)
        {
            await TryNotifyAsync(new StackNotificationWorkItem(stackStreamManager, stack), ct);
        }
    }

    private async Task TryNotifyAsync(INotificationWorkItem notification, CancellationToken cancellationToken)
    {
        try
        {
            await notificationQueue.EnqueueAsync(notification, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to enqueue container command completion notification");
        }
    }

    private bool IsOwnedClaim(
        ResourceControlState state,
        Guid? owner,
        long rowVersion,
        long expectedRowVersion)
        => state == ResourceControlState.Processing &&
           owner == controlTriggeredBy &&
           rowVersion == expectedRowVersion;
}

internal sealed record ProcessedResources(
    List<Container> Containers,
    List<Deployment> Deployments,
    List<Stack> Stacks,
    bool HasConflict = false)
{
    internal static ProcessedResources Conflict { get; } = new([], [], [], HasConflict: true);
}
