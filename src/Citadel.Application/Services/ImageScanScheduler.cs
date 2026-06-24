using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal interface IImageScanScheduler
{
    Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<ManualStackImageCheck>> LoadManualStackChecksAsync(CancellationToken cancellationToken);
}

internal sealed class ImageScanScheduler(IServiceScopeFactory scopeFactory) : IImageScanScheduler
{
    public async Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken)
    {
        var deploymentChecks = await LoadDeploymentChecksAsync(cancellationToken);
        var stackChecks = await LoadManualStackChecksAsync(cancellationToken);
        var registriesById = await LoadRegistriesAsync(deploymentChecks, stackChecks, cancellationToken);
        var scanTasks = new Dictionary<ImageKey, ImageScanTask>();

        foreach (var check in deploymentChecks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry) || registry.Status != RegistryStatus.Active)
                continue;

            scanTasks.TryAdd(check.Key, new ImageScanTask(check.Key, check.Deployment.PlatformId, registry));
        }

        foreach (var check in stackChecks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry) || registry.Status != RegistryStatus.Active)
                continue;

            scanTasks.TryAdd(check.Key, new ImageScanTask(check.Key, check.Stack.CurrentStackRelease!.PlatformId, registry));
        }

        return scanTasks.Values;
    }

    public async Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken)
    {
        var deployments = await LoadDeploymentsAsync(cancellationToken);
        var checks = new List<DeploymentImageCheck>(deployments.Count());

        foreach (var deployment in deployments)
        {
            if (TryBuildDeploymentImageCheck(deployment, out var check) && check != null)
            {
                checks.Add(check);
            }
        }

        return checks;
    }

    public async Task<IReadOnlyCollection<ManualStackImageCheck>> LoadManualStackChecksAsync(CancellationToken cancellationToken)
    {
        var stacks = await LoadStacksAsync(cancellationToken);
        var checks = new List<ManualStackImageCheck>();

        foreach (var stack in stacks)
        {
            checks.AddRange(BuildManualStackImageChecks(stack));
        }

        return checks;
    }

    private async Task<IEnumerable<Deployment>> LoadDeploymentsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Deployments.GetAllAsync(cancellationToken) ?? [];
    }

    private async Task<IEnumerable<Stack>> LoadStacksAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Stacks.GetAllAsync(cancellationToken) ?? [];
    }

    private async Task<IReadOnlyDictionary<Guid, Registry>> LoadRegistriesAsync(
        IReadOnlyCollection<DeploymentImageCheck> deploymentChecks,
        IReadOnlyCollection<ManualStackImageCheck> stackChecks,
        CancellationToken cancellationToken)
    {
        var registryIds = deploymentChecks
            .Select(c => c.Key.RegistryId)
            .Concat(stackChecks.Select(c => c.Key.RegistryId))
            .Distinct()
            .ToArray();

        if (registryIds.Length == 0)
            return new Dictionary<Guid, Registry>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(registryIds, cancellationToken) ?? [];
        return registries.ToDictionary(r => r.Id);
    }

    private static bool TryBuildDeploymentImageCheck(Deployment deployment, out DeploymentImageCheck? check)
    {
        check = default;

        if (deployment.Spec?.UpdateBehavior == UpdateBehavior.Disabled)
            return false;

        if (deployment.Spec?.Image is not ExternalImage deployedImage)
            return false;

        if (!Helpers.TrySplitImageTag(deployedImage.ImageTag, out var repository, out var tag))
            return false;

        check = new DeploymentImageCheck(
            deployment,
            deployedImage,
            new ImageKey(deployedImage.RegistryId, repository, tag));

        return true;
    }

    private static IEnumerable<ManualStackImageCheck> BuildManualStackImageChecks(Stack stack)
    {
        if (stack is { StackSource: not StackSource.WebEditor })
            yield break;

        if (stack.CurrentStackRelease is not { Spec: ManualStack manualStack })
            yield break;

        if (manualStack.UpdateBehavior == StackUpdateBehavior.Disabled)
            yield break;

        if (manualStack.RegistryId is not Guid registryId || registryId == Guid.Empty)
            yield break;

        if (stack.ControlState == ResourceControlState.Processing)
            yield break;

        if (stack.CurrentStackRelease.Status is not (StackReleaseStatus.Healthy or StackReleaseStatus.Degraded))
            yield break;

        foreach (var service in StackComposeParser.ParseServices(stack.Id, stack.CurrentStackReleaseId, manualStack.ComposeFile).Values)
        {
            if (string.IsNullOrWhiteSpace(service.Image))
                continue;

            if (!Helpers.TrySplitImageTag(service.Image, out var repository, out var tag))
                continue;

            yield return new ManualStackImageCheck(
                stack,
                service.ServiceName,
                service.Image,
                new ImageKey(registryId, repository, tag));
        }
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
