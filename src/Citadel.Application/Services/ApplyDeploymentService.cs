using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal interface IApplyDeploymentService
{
    IAsyncEnumerable<DeploymentStreamItem> ApplyAsync(Guid deploymentId, Guid actorId, bool recreate, CancellationToken ct);
}

internal sealed class ApplyDeploymentService(
    IDbWorkQueue dbWorkQueue,
    IServiceScopeFactory scopeFactory,
    IPullImageService pullImageService,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformCache,
    IDeploymentStreamManager deploymentHub,
    IActivityStreamManager activityHub,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IDeploymentConnector> deploymentConnectorFactory) : IApplyDeploymentService
{
    public async IAsyncEnumerable<DeploymentStreamItem> ApplyAsync(Guid deploymentId, Guid actorId, bool recreate, [System.Runtime.CompilerServices.EnumeratorCancellation] CancellationToken ct)
    {
        var deployment = await LoadDeployment(deploymentId, ct);
        if (deployment is null)
        {
            yield return Error(404, $"Deployment {deploymentId} not found.");
            yield break;
        }

        if (deployment.Spec is null)
        {
            yield return Error(400, "Deployment spec missing.");
            yield break;
        }

        if (!platformCache.TryGetCacheEntry(deployment.PlatformId, out var platform, out _))
        {
            var message = "Platform not found or disconnected.";
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct);
            yield return Error(404, message);
            yield break;
        }

        await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Applying, null, ct);

        string? imageId = null;
        string? digest = null;

        if (deployment.Spec.Image is LocalImage local)
        {
            imageId = local.ImageId;
        }
        else if (deployment.Spec.Image is ExternalImage external)
        {
            yield return new DeploymentStreamItem(
                ProgressMessage: $"Pulling image {external.ImageTag}");

            await foreach (var item in pullImageService.PullAsync(
                new PullImageService.PullImageInput(
                    PlatformId: platform.Id,
                    ImageTag: external.ImageTag,
                    RegistryId: external.RegistryId),
                ct))
            {
                yield return new DeploymentStreamItem(
                    Id: item.Id,
                    Status: item.Status,
                    Stream: item.Stream,
                    ProgressMessage: item.ProgressMessage,
                    ErrorMessage: item.ErrorMessage,
                    Progress: item.Progress,
                    Error: item.Error is not null
                        ? new DeploymentApplyError(item.Error.Code, item.Error.Message)
                        : null
                );

                if (!string.IsNullOrEmpty(item.ErrorMessage))
                {
                    await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, item.ErrorMessage, ct);
                    yield break;
                }

                if (!string.IsNullOrEmpty(item.DockerImageId))
                {
                    digest = item.Digest;
                    imageId = item.DockerImageId;
                }
            }
        }

        if (string.IsNullOrEmpty(imageId))
        {
            var message = "Image ID could not be resolved.";
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct);
            yield return Error(400, message);
            yield break;
        }

        if (recreate)
        {
            var (deletedContainerId, errorMessage) = await DeleteContainer(deployment.Id, platform, ct);
            if (!string.IsNullOrEmpty(errorMessage))
            {
                yield return Error(500, errorMessage);
                yield break;
            }

            if (!string.IsNullOrEmpty(deletedContainerId))
            {
                yield return Info($"Container deleted: {deletedContainerId}");
            }
        }

        yield return Info($"Applying deployment to {platform.Address}...");

        var connector = deploymentConnectorFactory.GetConnector(platform.ConnectorType);
        var commandToApply = BuildApplyCommand(deployment, platform.Address, imageId);

        var result = await connector.ApplyDeploymentAsync(commandToApply, ct);
        if (!result.IsSuccess(out var deploymentResult, out var error))
        {
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, error.Message, ct);
            yield return Error(500, error.Message);
            yield break;
        }

        yield return Info($"Container created: {deploymentResult.ContainerId}");

        if (deploymentResult.DeployedContainerState != DeployedContainerState.Running)
        {
            var message = $"Deployment failed: container did not start successfully - Container state: {deploymentResult.DeployedContainerState}";
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct);
            yield return Error(422, message);
            yield break;
        }

        await dbWorkQueue.EnqueueAsync(
            new DeploymentSucceededWorkItem(
                deployment.Id,
                actorId,
                deploymentResult.ContainerId,
                digest ?? "",
                deploymentHub,
                activityHub,
                notificationQueue),
            ct);

        yield return Info("✅ Deployment is now running.");
    }

    private static ApplyDeploymentCommand BuildApplyCommand(Deployment deployment, string platformAddress, string imageId)
    {
        var rs = deployment.Spec!.ResourceSpec;

        var normalized = rs != null
            ? rs with
            {
                NanoCpus = rs?.NanoCpus is > 0 ? (long)(rs.NanoCpus.Value * 1_000_000_000) : 0,
                MemoryLimit = rs?.MemoryLimit is > 0 ? rs.MemoryLimit.Value * 1024 * 1024 : 0
            }
            : ResourceSpec.Empty;

        return new ApplyDeploymentCommand(
            PlatformAddress: platformAddress,
            Name: deployment.Name,
            ImageId: imageId,
            Spec: deployment.Spec with { ResourceSpec = normalized });
    }

    private static DeploymentStreamItem Info(string message)
        => new(ProgressMessage: message);

    private static DeploymentStreamItem Error(int code, string message)
        => new(ErrorMessage: $"❌ {message}", Error: new DeploymentApplyError(code, $"❌ {message}"));

    private async Task EnqueueStatus(Guid deploymentId, Guid actorId, DeploymentStatus status, string? message, CancellationToken ct, bool notifyClients = true)
    {
        await dbWorkQueue.EnqueueAsync(
            new UpdateDeploymentStatusWorkItem(
                deploymentId,
                actorId,
                status,
                message,
                deploymentHub,
                activityHub,
                notificationQueue,
                notifyClients),
            ct);
    }

    private async Task<Deployment?> LoadDeployment(Guid id, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Deployments.GetAsync(id, ct);
    }

    private async Task<(string? ContainerId, string? error)> DeleteContainer(Guid deploymentId, PlatformCacheEntry platform, CancellationToken ct)
    {
        var container = await GetContainer(deploymentId, ct);
        if (container == null) return (null, null);

        var connector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var commandToApply = new DeleteContainerCommand([container.DockerContainerId], platform.Address, true, true, false);

        var result = await connector.DeleteAsync(commandToApply, ct);
        if (result.IsFailure(out var error))
        {
            return (null, error.Message);
        }

        return (container.DockerContainerId, null);
    }

    private async Task<Container?> GetContainer(Guid deploymentId, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Containers.GetByDeploymentIdAsync(deploymentId, ct);
    }
}


internal sealed class UpdateDeploymentStatusWorkItem(Guid deploymentId, Guid actorId, DeploymentStatus status, string? message, IDeploymentStreamManager deploymentHub,
    IActivityStreamManager activityHub, INotificationQueue notificationQueue, bool notifyClients = true) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, ct);
        if (deployment is null) return;

        deployment.PartialUpdate(status: status);
        await uow.Deployments.UpdateAsync(deployment, ct);

        // Add activity event
        ActivityEvent? activity = null;
        if (status == DeploymentStatus.Failed)
        {
            activity = new ActivityEvent(
                            actorId: actorId,
                            resourceId: deployment.Id,
                            platformId: deployment.PlatformId,
                            resourceName: deployment.Name,
                            status: ActivityStatus.Failure,
                            eventType: ActivityEventType.DeploymentApplied,
                            info: new DeploymentApplied(null, null, message)
                            );

            await uow.ActivityEventRepository.AddAsync(activity, ct);
        }

        await uow.CommitAsync(ct);

        // Push notifications
        if (notifyClients)
        {
            var deploymentWorkItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
            await notificationQueue.EnqueueAsync(deploymentWorkItem, ct);
        }

        if (activity is not null && notifyClients)
        {
            var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
            await notificationQueue.EnqueueAsync(activityWorkItem, ct);
        }
    }
}

internal sealed class DeploymentSucceededWorkItem(Guid deploymentId, Guid actorId, string containerId, string imageDigest,
    IDeploymentStreamManager deploymentHub, IActivityStreamManager activityHub, INotificationQueue notificationQueue) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, ct);
        var container = await uow.Containers.GetByIdAsync(containerId, ct);

        if (deployment is null || container is null) return;

        container.PartialUpdate(deploymentId: deployment.Id);
        deployment.PartialUpdate(status: DeploymentStatus.Healthy, container: container);
        // Update the deployed image with the resolved digest 
        if (!string.IsNullOrEmpty(imageDigest) && deployment.Spec?.Image is ExternalImage extImage)
        {
            deployment.PartialUpdate(spec: deployment.Spec with { Image = new ExternalImage(
                RegistryId: extImage.RegistryId,
                ImageTag: extImage.ImageTag,
                ResolvedDigest: imageDigest
                ) });
        }

        await uow.Containers.UpdateAsync(container, ct);
        await uow.Deployments.UpdateAsync(deployment, ct);

        // Add activity event
        var activity = new ActivityEvent(
                        actorId: actorId,
                        resourceId: deployment.Id,
                        platformId: deployment.PlatformId,
                        resourceName: deployment.Name,
                        status: ActivityStatus.Success,
                        eventType: ActivityEventType.DeploymentApplied,
                        info: new DeploymentApplied(deployment.ToSnapshot(), [containerId], null)
                        );

        await uow.ActivityEventRepository.AddAsync(activity, ct);

        await uow.CommitAsync(ct);

        // Push notifications
        var deploymentWorkItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        await notificationQueue.EnqueueAsync(deploymentWorkItem, ct);

        var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
        await notificationQueue.EnqueueAsync(activityWorkItem, ct);
    }
}