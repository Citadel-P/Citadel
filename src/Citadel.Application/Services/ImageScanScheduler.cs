using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Registries;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal interface IImageScanScheduler
{
    Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken);
}

internal sealed class ImageScanScheduler(IServiceScopeFactory scopeFactory) : IImageScanScheduler
{
    public async Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken)
    {
        var checks = await LoadDeploymentChecksAsync(cancellationToken);
        var registriesById = await LoadRegistriesAsync(checks, cancellationToken);
        var scanTasks = new Dictionary<ImageKey, ImageScanTask>();

        foreach (var check in checks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry) || registry.Status != RegistryStatus.Active)
                continue;

            scanTasks.TryAdd(check.Key, new ImageScanTask(check.Key, check.Deployment.PlatformId, registry));
        }

        return scanTasks.Values;
    }

    public async Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken)
    {
        var deployments = await LoadDeploymentsAsync(cancellationToken);
        var checks = new List<DeploymentImageCheck>(deployments.Count);

        foreach (var deployment in deployments)
        {
            if (TryBuildDeploymentImageCheck(deployment, out var check) && check != null)
            {
                checks.Add(check);
            }
        }

        return checks;
    }

    private async Task<IReadOnlyList<Deployment>> LoadDeploymentsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return [.. (await uow.Deployments.GetAllAsync(cancellationToken) ?? [])];
    }

    private async Task<IReadOnlyDictionary<Guid, Registry>> LoadRegistriesAsync(IReadOnlyCollection<DeploymentImageCheck> checks, CancellationToken cancellationToken)
    {
        var registryIds = checks
            .Select(c => c.Key.RegistryId)
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

        if (!TrySplitImageTag(deployedImage.ImageTag, out var repository, out var tag))
            return false;

        check = new DeploymentImageCheck(
            deployment,
            deployedImage,
            new ImageKey(deployedImage.RegistryId, repository, tag));

        return true;
    }

    private static bool TrySplitImageTag(string imageTag, out string repository, out string tag)
    {
        repository = string.Empty;
        tag = string.Empty;

        if (string.IsNullOrWhiteSpace(imageTag) || imageTag.Contains('@'))
            return false;

        var lastSlashIndex = imageTag.LastIndexOf('/');
        var lastColonIndex = imageTag.LastIndexOf(':');

        if (lastColonIndex > lastSlashIndex)
        {
            repository = imageTag[..lastColonIndex];
            tag = imageTag[(lastColonIndex + 1)..];
        }
        else
        {
            repository = imageTag;
            tag = "latest";
        }

        return !string.IsNullOrWhiteSpace(repository) && !string.IsNullOrWhiteSpace(tag);
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
