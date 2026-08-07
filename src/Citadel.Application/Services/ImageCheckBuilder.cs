using Domain;
using Domain.Entities.Deployments;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using YamlDotNet.Core;

namespace Application.Services;

internal enum ImageCheckMode
{
    Scheduled,
    OnDemand
}

internal interface IImageCheckBuilder
{
    Result<DeploymentImageCheck> BuildDeploymentCheck(Deployment deployment, ImageCheckMode mode);
    Result<IReadOnlyList<ManualStackImageCheck>> BuildManualStackChecks(Stack stack, ImageCheckMode mode);
    Result<SwarmServiceImageCheck> BuildSwarmServiceCheck(SwarmService service, ImageCheckMode mode);
    Result<ImageScanTask> BuildScanTask(ImageKey key, Guid platformId, Registry registry);
}

internal sealed class ImageCheckBuilder : IImageCheckBuilder
{
    private static readonly HashSet<StackReleaseStatus> OnDemandStackStatuses =
    [
        StackReleaseStatus.Healthy,
        StackReleaseStatus.Degraded,
        StackReleaseStatus.Stopped,
        StackReleaseStatus.Paused
    ];

    public Result<DeploymentImageCheck> BuildDeploymentCheck(Deployment deployment, ImageCheckMode mode)
    {
        if (mode == ImageCheckMode.OnDemand && deployment.ControlState == ResourceControlState.Processing)
        {
            return Result.Failure<DeploymentImageCheck>(
                new ConflictError("The deployment is currently processing another operation."));
        }

        if (deployment.Spec is null)
        {
            return Result.Failure<DeploymentImageCheck>(
                new BadRequestError("The deployment has no image configuration."));
        }

        if (mode == ImageCheckMode.Scheduled && deployment.Spec.UpdateBehavior == UpdateBehavior.Disabled)
        {
            return Result.Failure<DeploymentImageCheck>(
                new BadRequestError("Scheduled update checks are disabled for this deployment."));
        }

        if (deployment.Spec.Image is not ExternalImage deployedImage)
        {
            return Result.Failure<DeploymentImageCheck>(
                new BadRequestError("Only external tagged images support update checks."));
        }

        if (deployedImage.RegistryId == Guid.Empty)
        {
            return Result.Failure<DeploymentImageCheck>(
                new BadRequestError("The deployment has no registry configured."));
        }

        if (!Helpers.TrySplitImageTag(deployedImage.ImageTag, out var repository, out var tag))
        {
            return Result.Failure<DeploymentImageCheck>(
                new BadRequestError("The deployment image must use a supported tagged reference."));
        }

        if (mode == ImageCheckMode.OnDemand && string.IsNullOrWhiteSpace(deployedImage.ResolvedDigest))
        {
            return Result.Failure<DeploymentImageCheck>(
                new ConflictError("The deployment has no applied image digest to compare."));
        }

        return Result.Success(new DeploymentImageCheck(
            deployment,
            deployedImage,
            new ImageKey(deployedImage.RegistryId, repository, tag)));
    }

    public Result<IReadOnlyList<ManualStackImageCheck>> BuildManualStackChecks(Stack stack, ImageCheckMode mode)
    {
        if (stack.StackSource != StackSource.WebEditor
            || stack.CurrentStackRelease?.Spec is not ManualStack manualStack)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new BadRequestError("Only Web Editor stacks support image update checks."));
        }

        if (stack.ControlState == ResourceControlState.Processing)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new ConflictError("The stack is currently processing another operation."));
        }

        if (mode == ImageCheckMode.Scheduled && manualStack.UpdateBehavior == StackUpdateBehavior.Disabled)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new BadRequestError("Scheduled update checks are disabled for this stack."));
        }

        var allowedStatus = mode == ImageCheckMode.Scheduled
            ? stack.CurrentStackRelease.Status is StackReleaseStatus.Healthy or StackReleaseStatus.Degraded
            : OnDemandStackStatuses.Contains(stack.CurrentStackRelease.Status);
        if (!allowedStatus)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new ConflictError("The stack release state does not support update checks."));
        }

        if (manualStack.RegistryId is not Guid registryId || registryId == Guid.Empty)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new BadRequestError("The stack has no registry configured."));
        }

        IReadOnlyDictionary<string, StackComposeService> services;
        try
        {
            services = StackComposeParser.ParseServices(
                stack.Id,
                stack.CurrentStackReleaseId,
                manualStack.ComposeFile);
        }
        catch (YamlException)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new BadRequestError("The stack Compose configuration is invalid."));
        }

        var buildBoundServices = manualStack.BuildImageBindings?
            .Select(binding => binding.ServiceName)
            .Where(name => !string.IsNullOrWhiteSpace(name))
            .ToHashSet(StringComparer.OrdinalIgnoreCase)
            ?? [];
        var checks = new List<ManualStackImageCheck>();

        foreach (var service in services.Values)
        {
            if (string.IsNullOrWhiteSpace(service.Image)
                || buildBoundServices.Contains(service.ServiceName)
                || !Helpers.TrySplitImageTag(service.Image, out var repository, out var tag))
            {
                continue;
            }

            checks.Add(new ManualStackImageCheck(
                stack,
                service.ServiceName,
                service.Image,
                new ImageKey(registryId, repository, tag)));
        }

        if (checks.Count == 0)
        {
            return Result.Failure<IReadOnlyList<ManualStackImageCheck>>(
                new BadRequestError("The stack has no supported tagged service images to check."));
        }

        return Result.Success<IReadOnlyList<ManualStackImageCheck>>(checks);
    }

    public Result<SwarmServiceImageCheck> BuildSwarmServiceCheck(
        SwarmService service,
        ImageCheckMode mode)
    {
        if (service.ControlState == ResourceControlState.Processing)
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new ConflictError("The Service is currently processing another operation."));
        }

        if (mode == ImageCheckMode.Scheduled
            && service.Spec.UpdateBehavior == UpdateBehavior.Disabled)
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new BadRequestError("Scheduled update checks are disabled for this Service."));
        }

        if (service.Spec.Image is not SwarmExternalImage image)
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new BadRequestError("Only external tagged images support update checks."));
        }

        if (image.RegistryId == Guid.Empty)
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new BadRequestError("The Service has no Registry configured."));
        }

        if (!Helpers.TrySplitImageTag(image.ImageTag, out var repository, out var tag))
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new BadRequestError("The Service image must use a supported tagged reference."));
        }

        if (string.IsNullOrWhiteSpace(service.AppliedImageDigest))
        {
            return Result.Failure<SwarmServiceImageCheck>(
                new ConflictError("The Service has no applied image digest to compare."));
        }

        return Result.Success(new SwarmServiceImageCheck(
            service,
            image,
            new ImageKey(image.RegistryId, repository, tag)));
    }

    public Result<ImageScanTask> BuildScanTask(ImageKey key, Guid platformId, Registry registry)
    {
        if (registry.Status != RegistryStatus.Active)
        {
            return Result.Failure<ImageScanTask>(
                new ConflictError("The configured registry is not active."));
        }

        var registryHost = registry.RegistryHost.Contains("://", StringComparison.Ordinal)
            ? registry.RegistryHost
            : $"https://{registry.RegistryHost}";
        if (!Uri.TryCreate(registryHost, UriKind.Absolute, out var uri)
            || string.IsNullOrWhiteSpace(uri.Host))
        {
            return Result.Failure<ImageScanTask>(
                new BadRequestError("The configured registry host is invalid."));
        }

        return Result.Success(new ImageScanTask(key, platformId, registry));
    }
}

internal sealed record ImageScanTask(
    ImageKey Key,
    Guid PlatformId,
    Registry Registry);

internal sealed record DeploymentImageCheck(
    Deployment Deployment,
    ExternalImage DeployedImage,
    ImageKey Key);

internal sealed record ManualStackImageCheck(
    Stack Stack,
    string ServiceName,
    string ImageName,
    ImageKey Key);

internal sealed record SwarmServiceImageCheck(
    SwarmService Service,
    SwarmExternalImage Image,
    ImageKey Key);
