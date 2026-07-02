using Application.Features.Deployments.Notifications;
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
using System.Text.RegularExpressions;

namespace Application.Services;

internal interface IApplyDeploymentService
{
    IAsyncEnumerable<DeploymentStreamItem> ApplyAsync(Guid deploymentId, Guid actorId, bool recreate, CancellationToken ct);
}

internal sealed partial class ApplyDeploymentService(
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
    IConnectorFactory<IDeploymentConnector> deploymentConnectorFactory) : IApplyDeploymentService
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

        var injectedConfiguration = resolvedConfiguration.SelectEntries(GetReferencedConfigurationNames(deployment.Spec.EnvironmentVariables ?? []));
        var environmentResult = BuildDeploymentEnvironmentVariables(deployment, injectedConfiguration);
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

        await dbWorkQueue.EnqueueAsync(
            new DeploymentSucceededWorkItem(
                deployment.Id,
                actorId,
                deploymentResult.ContainerId,
                digest ?? "",
                autoUpdateState,
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

    private static Result<IReadOnlyList<string>> BuildDeploymentEnvironmentVariables(
        Deployment deployment,
        ResolvedResourceBindings configuration)
    {
        var configured = deployment.Spec?.EnvironmentVariables ?? [];
        if (configured.Count == 0)
            return Array.Empty<string>();

        var values = configuration.ToValueDictionary();
        var result = new List<string>(configured.Count);

        foreach (var rawLine in configured)
        {
            var line = rawLine.Trim();
            if (string.IsNullOrWhiteSpace(line) || line.StartsWith('#'))
                continue;

            var separator = line.IndexOf('=');
            if (separator < 0)
            {
                if (!IsValidEnvironmentName(line))
                    return Result.Failure<IReadOnlyList<string>>($"Deployment environment key '{line}' is not valid.");

                if (!values.TryGetValue(line, out var value))
                    return Result.Failure<IReadOnlyList<string>>($"Deployment environment key '{line}' is not defined in Variables.");

                result.Add($"{line}={value}");
                continue;
            }

            var name = line[..separator].Trim();
            var template = line[(separator + 1)..];
            if (!IsValidEnvironmentName(name))
                return Result.Failure<IReadOnlyList<string>>($"Deployment environment key '{name}' is not valid.");

            var valueResult = InterpolateEnvironmentTemplate(template, values);
            if (valueResult.IsFailure(out var interpolationError, out var interpolatedValue))
                return Result.Failure<IReadOnlyList<string>>(interpolationError.Message);

            result.Add($"{name}={interpolatedValue}");
        }

        return result;
    }

    private static IEnumerable<string> GetReferencedConfigurationNames(IEnumerable<string> configured)
    {
        foreach (var rawLine in configured)
        {
            var line = rawLine.Trim();
            if (string.IsNullOrWhiteSpace(line) || line.StartsWith('#'))
                continue;

            var separator = line.IndexOf('=');
            if (separator < 0)
            {
                yield return line;
                continue;
            }

            foreach (Match match in EnvironmentReferenceRegex().Matches(line[(separator + 1)..]))
            {
                yield return match.Groups["name"].Value;
            }
        }
    }

    private static Result<string> InterpolateEnvironmentTemplate(string template, IReadOnlyDictionary<string, string> values)
    {
        var missingName = string.Empty;
        var interpolated = EnvironmentReferenceRegex().Replace(template, match =>
        {
            var name = match.Groups["name"].Value;
            if (!values.TryGetValue(name, out var value))
            {
                missingName = name;
                return match.Value;
            }

            return value;
        });

        return string.IsNullOrEmpty(missingName)
            ? interpolated
            : Result.Failure<string>($"Deployment environment reference '{missingName}' is not defined in Variables.");
    }

    private static bool IsValidEnvironmentName(string name)
        => EnvironmentNameRegex().IsMatch(name);

    [GeneratedRegex("^[A-Za-z_][A-Za-z0-9_]*$", RegexOptions.Compiled)]
    private static partial Regex EnvironmentNameRegex();

    [GeneratedRegex(@"\$\{(?<name>[A-Za-z_][A-Za-z0-9_]*)\}", RegexOptions.Compiled)]
    private static partial Regex EnvironmentReferenceRegex();

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
