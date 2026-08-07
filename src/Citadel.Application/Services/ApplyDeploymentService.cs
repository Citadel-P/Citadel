using Application.Features.Deployments.Notifications;
using Application.Services.Builds;
using Application.Services.Alerts;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;

namespace Application.Services;

internal interface IApplyDeploymentService
{
    IAsyncEnumerable<DeploymentStreamItem> ApplyAsync(Guid deploymentId, Guid actorId, bool recreate, CancellationToken ct);
}

internal sealed class ApplyDeploymentService(
    IDbWorkQueue dbWorkQueue,
    IServiceScopeFactory scopeFactory,
    IPullImageService pullImageService,
    ImageDigestCache imageDigestCache,
    INotificationQueue notificationQueue,
    IPlatformContainerCache platformCache,
    IResourceBindingResolver configurationResolver,
    ISecretRedactor secretRedactor,
    IAlertService alertService,
    IDeploymentStreamManager deploymentHub,
    IActivityStreamManager activityHub,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IDeploymentConnector> deploymentConnectorFactory,
    IBuildImageResolver buildImageResolver) : IApplyDeploymentService
{
    public async IAsyncEnumerable<DeploymentStreamItem> ApplyAsync(Guid deploymentId, Guid actorId, bool recreate, [EnumeratorCancellation] CancellationToken ct)
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

        if (deployment.Platform?.PlatformDescriptor.Type == PlatformType.DockerSwarm)
        {
            yield return Error(501, "Applying Docker Swarm services is not available yet.");
            yield break;
        }

        var existingContainer = await GetContainer(deployment.Id, ct);
        if (existingContainer is not null && existingContainer.PlatformId != deployment.PlatformId)
        {
            yield return Error(
                409,
                "This deployment references a different platform than its existing container. Duplicate it on the target platform or restore the original platform before applying.");
            yield break;
        }

        if (!platformCache.TryGetCacheEntry(deployment.PlatformId, out var platform, out _))
        {
            var message = "Platform not found or disconnected.";
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct: ct);
            yield return Error(404, message);
            yield break;
        }

        await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Applying, null, ct: ct);

        string? imageId = null;
        string? digest = null;
        ResolvedBuildImage? resolvedBuildImage = null;

        ExternalImage? pulledExternalImage = null;

        if (deployment.Spec.Image is LocalImage local)
        {
            imageId = await ResolveLocalImageIdAsync(local.ImageId, deployment.PlatformId, ct);
        }
        else if (deployment.Spec.Image is ExternalImage external)
        {
            pulledExternalImage = external;
        }
        else if (deployment.Spec.Image is BuildImage buildImage)
        {
            var resolvedBuild = await buildImageResolver.ResolveAsync(
                buildImage.BuildProjectId,
                buildImage.ResolvedImageReference,
                buildImage.ResolvedDigest,
                buildImage.ResolvedBuildRunId,
                ct);
            if (resolvedBuild.IsFailure(out var resolvedBuildError, out var resolved))
            {
                var message = resolvedBuildError.Message;
                await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct: ct);
                yield return Error(400, message);
                yield break;
            }

            pulledExternalImage = new ExternalImage(
                resolved.RegistryId,
                resolved.ImageReference,
                resolved.Digest);
            resolvedBuildImage = resolved;

            yield return Info(resolved.RunId is Guid runId
                ? $"Resolved build \"{resolved.ProjectName}\" from run {runId}."
                : $"Resolved build \"{resolved.ProjectName}\" from its stored artifact.");
        }

        if (pulledExternalImage is not null)
        {
            yield return new DeploymentStreamItem(
                ProgressMessage: $"Pulling image {pulledExternalImage.ImageTag}");

            await foreach (var item in pullImageService.PullAsync(
                new PullImageService.PullImageInput(
                    PlatformId: platform.Id,
                    ImageTag: pulledExternalImage.ImageTag,
                    RegistryId: pulledExternalImage.RegistryId),
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
                    await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, item.ErrorMessage, ct: ct);
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
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, ct: ct);
            yield return Error(400, message);
            yield break;
        }

        if (recreate)
        {
            var (deletedContainerId, errorMessage) = await DeleteContainer(existingContainer, ct);
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

        yield return Info("Resolving deployment variables and secrets...");

        var configurationResult = await configurationResolver.ResolveAsync(ResourceBindingScope.Deployment, deployment.Id, ct);
        if (configurationResult.IsFailure(out var configurationError, out var resolvedConfiguration))
        {
            var safeMessage = configurationError.Message;
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, safeMessage, ct: ct);
            await ProcessConfigurationFailureAlertAsync(deployment.Id, deployment.Name, safeMessage, ct);
            yield return Error(400, safeMessage);
            yield break;
        }

        var configuredEnvironment = deployment.Spec.EnvironmentVariables ?? [];
        var injectedConfiguration = resolvedConfiguration.SelectEntries(
            EnvironmentVariableResolver.GetReferencedNames(configuredEnvironment));
        var environmentResult = EnvironmentVariableResolver.Build(
            configuredEnvironment,
            injectedConfiguration,
            "Deployment");
        if (environmentResult.IsFailure(out var environmentError, out var environmentVariables))
        {
            var safeMessage = environmentError.Message;
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, safeMessage, resourceBindings: injectedConfiguration.SnapshotEntries, ct: ct);
            yield return Error(400, safeMessage);
            yield break;
        }

        yield return Info(ResourceBindingApplyMessageBuilder.BuildDeploymentEnvironmentMessage(injectedConfiguration));

        yield return Info($"Applying deployment to {platform.Address}...");

        var connector = deploymentConnectorFactory.GetConnector(platform.ConnectorType);
        var commandToApply = BuildApplyCommand(deployment, platform.Address, imageId, environmentVariables);

        var result = await connector.ApplyDeploymentAsync(commandToApply, ct);
        if (!result.IsSuccess(out var deploymentResult, out var error))
        {
            var safeMessage = secretRedactor.Redact(error.Message, injectedConfiguration.RedactionValues);
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, safeMessage, resourceBindings: injectedConfiguration.SnapshotEntries, ct: ct);
            yield return Error(500, safeMessage);
            yield break;
        }

        yield return Info($"Container created: {deploymentResult.ContainerId}");

        var autoUpdateState = ResolveAutoUpdateState(deployment, digest);

        if (deploymentResult.DeployedContainerState != DeployedContainerState.Running)
        {
            var message = $"Deployment failed: container did not start successfully - Container state: {deploymentResult.DeployedContainerState}";
            await EnqueueStatus(deployment.Id, actorId, DeploymentStatus.Failed, message, deploymentResult.ContainerId, autoUpdateState, injectedConfiguration.SnapshotEntries, ct);
            yield return Error(422, message);
            yield break;
        }

        await dbWorkQueue.EnqueueAndWaitAsync(
            new DeploymentSucceededWorkItem(
                deployment.Id,
                actorId,
                deploymentResult.ContainerId,
                digest ?? "",
                autoUpdateState,
                resolvedBuildImage,
                deploymentHub,
                activityHub,
                notificationQueue,
                injectedConfiguration.SnapshotEntries),
            ct);

        yield return Info("Deployment is now running.");
    }

    private static ApplyDeploymentCommand BuildApplyCommand(
        Deployment deployment,
        string platformAddress,
        string imageId,
        IReadOnlyList<string> environmentVariables)
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
            Spec: deployment.Spec with { ResourceSpec = normalized },
            EnvironmentVariables: environmentVariables);
    }

    private Task ProcessConfigurationFailureAlertAsync(Guid deploymentId, string deploymentName, string reason, CancellationToken ct)
    {
        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            DeploymentConfigurationFailures:
            [
                new DeploymentConfigurationResolutionFailureAlertSnapshot(
                    deploymentId,
                    deploymentName,
                    reason)
            ]);

        return alertService.ProcessAsync(AlertType.DeploymentConfigurationResolutionFailed, context, ct);
    }

    private static DeploymentStreamItem Info(string message)
        => new(ProgressMessage: message);

    private static DeploymentStreamItem Error(int code, string message)
        => new(ErrorMessage: message, Error: new DeploymentApplyError(code, message));

    private AutoUpdateState? ResolveAutoUpdateState(Deployment deployment, string? imageDigest)
    {
        if (string.IsNullOrEmpty(imageDigest)
            || deployment.Spec?.Image is not ExternalImage externalImage
            || deployment.Spec.UpdateBehavior == UpdateBehavior.Disabled
            || !Helpers.TrySplitImageTag(externalImage.ImageTag, out var repository, out var tag))
        {
            return null;
        }

        var remoteDigest = imageDigest;
        var status = AutoUpdateStatus.UpToDate;

        if (imageDigestCache.TryGet(new ImageKey(externalImage.RegistryId, repository, tag), out var digestEntry))
        {
            remoteDigest = digestEntry.Digest;
            status = string.Equals(remoteDigest, imageDigest, StringComparison.OrdinalIgnoreCase)
                ? AutoUpdateStatus.UpToDate
                : AutoUpdateStatus.UpdateAvailable;
        }

        return new AutoUpdateState(
            LastCheckedAt: DateTime.UtcNow,
            Status: status,
            CurrentDigest: imageDigest,
            RemoteDigest: remoteDigest);
    }

    private ValueTask EnqueueStatus(
        Guid deploymentId,
        Guid actorId,
        DeploymentStatus status,
        string? message,
        string? containerId = null,
        AutoUpdateState? autoUpdateState = null,
        IReadOnlyList<ResourceBindingSnapshot>? resourceBindings = null,
        CancellationToken ct = default)
        => dbWorkQueue.EnqueueAsync(
            new UpdateDeploymentStatusWorkItem(
                deploymentId,
                actorId,
                status,
                message,
                containerId,
                autoUpdateState,
                deploymentHub,
                activityHub,
                notificationQueue,
                resourceBindings),
            ct);
    
    private async Task<Deployment?> LoadDeployment(Guid id, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Deployments.GetAsync(id, ct);
    }

    private async Task<string?> ResolveLocalImageIdAsync(
        string imageId,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        if (!Guid.TryParse(imageId, out var localImageId))
            return imageId;

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = await uow.Images.GetByIdAsync(localImageId, platformId, cancellationToken);
        return image?.DockerImageId;
    }

    private async Task<(string? ContainerId, string? error)> DeleteContainer(Container? container, CancellationToken ct)
    {
        if (container == null) return (null, null);

        if (!platformCache.TryGetCacheEntry(container.PlatformId, out var appliedPlatform, out _))
        {
            return (null, "The deployment's current platform is unavailable. Restore that platform before moving the deployment.");
        }

        var connector = containerConnectorFactory.GetConnector(appliedPlatform.ConnectorType);
        var commandToApply = new DeleteContainerCommand([container.DockerContainerId], appliedPlatform.Address, true, true, false);

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

internal sealed class UpdateDeploymentStatusWorkItem(Guid deploymentId, Guid actorId, DeploymentStatus status, string? message, string? containerId, AutoUpdateState? autoUpdateState, IDeploymentStreamManager deploymentHub,
    IActivityStreamManager activityHub, INotificationQueue notificationQueue, IReadOnlyList<ResourceBindingSnapshot>? resourceBindings) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, ct);
        if (deployment is null) return;

        Container? container = null;
        if (!string.IsNullOrEmpty(containerId))
        {
            container = await uow.Containers.GetByIdAsync(containerId, ct);
            if (container is not null)
            {
                container.PartialUpdate(deploymentId: deployment.Id);
                deployment.PartialUpdate(container: container);
            }
        }

        deployment.PartialUpdate(status: status, autoUpdateState: autoUpdateState);
        await uow.Deployments.UpdateAsync(deployment, ct);
        if (container != null) await uow.Containers.UpdateAsync(container, ct);

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
                            info: new DeploymentApplied(deployment.ToSnapshot(), new DeploymentResultSnapshot(null, message, resourceBindings))
                            );

            await uow.ActivityEventRepository.AddAsync(activity, ct);
            deployment.AssignActivityEvent(activity);
        }

        await uow.CommitAsync(ct);

        // Push notifications
        
        var deploymentWorkItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        await notificationQueue.EnqueueAsync(deploymentWorkItem, ct);

        if (activity is not null)
        {
            var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
            await notificationQueue.EnqueueAsync(activityWorkItem, ct);
        }
    }
}

internal sealed class DeploymentSucceededWorkItem(
    Guid deploymentId, Guid actorId, string containerId, string imageDigest, AutoUpdateState? autoUpdateState,
    ResolvedBuildImage? appliedBuildImage,
    IDeploymentStreamManager deploymentHub, IActivityStreamManager activityHub, 
    INotificationQueue notificationQueue,
    IReadOnlyList<ResourceBindingSnapshot>? resourceBindings) : IDbWorkItem
{
    public async Task ExecuteAsync(IUnitOfWork uow, CancellationToken ct)
    {
        var deployment = await uow.Deployments.GetAsync(deploymentId, ct);
        var container = await uow.Containers.GetByIdAsync(containerId, ct);

        if (deployment is null || container is null) return;

        container.PartialUpdate(deploymentId: deployment.Id);
        deployment.PartialUpdate(status: DeploymentStatus.Healthy, container: container, autoUpdateState: autoUpdateState);

        if (!string.IsNullOrEmpty(imageDigest) && deployment.Spec?.Image is ExternalImage extImage)
        {
            deployment.PartialUpdate(
                spec: deployment.Spec with { Image = new ExternalImage(
                    RegistryId: extImage.RegistryId,
                    ImageTag: extImage.ImageTag,
                    ResolvedDigest: imageDigest)
                });
        }
        else if (appliedBuildImage is not null
                 && deployment.Spec?.Image is BuildImage buildImage
                 && buildImage.BuildProjectId == appliedBuildImage.ProjectId)
        {
            var appliedDigest = string.IsNullOrEmpty(imageDigest)
                ? appliedBuildImage.Digest
                : imageDigest;
            var hasResolvedArtifact = !string.IsNullOrWhiteSpace(buildImage.ResolvedImageReference);

            deployment.PartialUpdate(
                spec: deployment.Spec with
                {
                    Image = buildImage with
                    {
                        ResolvedImageReference = hasResolvedArtifact
                            ? buildImage.ResolvedImageReference
                            : appliedBuildImage.ImageReference,
                        ResolvedDigest = hasResolvedArtifact
                            ? buildImage.ResolvedDigest
                            : appliedDigest,
                        ResolvedBuildRunId = hasResolvedArtifact
                            ? buildImage.ResolvedBuildRunId
                            : appliedBuildImage.RunId,
                        AppliedImageReference = appliedBuildImage.ImageReference,
                        AppliedDigest = appliedDigest,
                        AppliedBuildRunId = appliedBuildImage.RunId,
                        AppliedAt = DateTimeOffset.UtcNow
                    }
                });
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
                        info: new DeploymentApplied(deployment.ToSnapshot(), new DeploymentResultSnapshot([containerId], ResourceBindings: resourceBindings))
                        );

        await uow.ActivityEventRepository.AddAsync(activity, ct);
        deployment.AssignActivityEvent(activity);

        await uow.CommitAsync(ct);

        // Push notifications
        var deploymentWorkItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        await notificationQueue.EnqueueAsync(deploymentWorkItem, ct);

        var activityWorkItem = new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(uow, ct));
        await notificationQueue.EnqueueAsync(activityWorkItem, ct);
    }
}
