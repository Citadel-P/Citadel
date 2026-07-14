using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Backups;

internal sealed class DeploymentBackupVolumeResolver(
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> containerConnectorFactory)
    : IDeploymentBackupVolumeResolver
{
    public async Task<Result<DeploymentBackupVolumeResolution>> ResolveAsync(
        Guid deploymentId,
        CancellationToken cancellationToken)
    {
        var inputResult = await LoadInputAsync(deploymentId, cancellationToken);
        if (!inputResult.IsSuccess(out var input, out var inputError))
            return Result.Failure<DeploymentBackupVolumeResolution>(inputError);

        var warnings = new List<string>(input.Warnings);
        var volumes = new Dictionary<string, ResolvedStackBackupVolume>(StringComparer.Ordinal);

        if (input.PlatformCache is not null)
        {
            if (input.Container is null)
            {
                warnings.Add("No synchronized container was found for this deployment. Deployment volume settings are used as a fallback.");
            }
            else
            {
                var connector = containerConnectorFactory.GetConnector(input.PlatformCache.ConnectorType);
                var inspect = await connector.InspectAsync(
                    new InspectContainerCommand(input.PlatformCache.Address, input.Container.DockerContainerId),
                    cancellationToken);

                if (!inspect.IsSuccess(out var info, out var inspectError))
                {
                    warnings.Add($"Could not inspect deployment container {input.Container.Name}: {inspectError.Message}");
                }
                else
                {
                    foreach (var mount in info.Mounts)
                    {
                        if (!string.Equals(mount.Type, "volume", StringComparison.OrdinalIgnoreCase)
                            || string.IsNullOrWhiteSpace(mount.Name))
                        {
                            continue;
                        }

                        var kind = input.DeclaredVolumeReferences.Contains(mount.Name)
                            ? StackVolumeKind.DeclaredNamed
                            : StackVolumeKind.AnonymousNamed;
                        AddVolume(volumes, input.PlatformId, mount.Name, kind);
                    }
                }
            }
        }

        if (volumes.Count == 0)
        {
            foreach (var volumeName in input.DeclaredVolumeReferences)
                AddVolume(volumes, input.PlatformId, volumeName, StackVolumeKind.DeclaredNamed);
        }

        if (input.HasAnonymousVolumes && volumes.Values.All(static volume => volume.Kind != StackVolumeKind.AnonymousNamed))
        {
            warnings.Add("The deployment declares anonymous volumes. Their runtime names can only be resolved after the deployment container is synchronized and inspectable.");
        }

        var resolved = volumes.Values
            .OrderBy(static volume => volume.VolumeName, StringComparer.Ordinal)
            .ToArray();

        if (resolved.Length == 0)
            warnings.Add("No Docker named volumes were resolved for this deployment.");

        return Result.Success(new DeploymentBackupVolumeResolution(
            input.DeploymentId,
            input.DeploymentName,
            input.PlatformId,
            input.PlatformName,
            input.PlatformStatus,
            resolved,
            [.. warnings.Distinct(StringComparer.Ordinal)]));
    }

    private async Task<Result<DeploymentBackupVolumeInput>> LoadInputAsync(
        Guid deploymentId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var deployment = await unitOfWork.Deployments.GetAsync(deploymentId, cancellationToken);
        if (deployment is null)
            return Result.Failure<DeploymentBackupVolumeInput>(new NotFoundError("Deployment not found."));

        var platform = await unitOfWork.Platforms.GetByIdAsync(deployment.PlatformId, cancellationToken);
        var warnings = new List<string>();
        PlatformCacheEntry? platformCache = null;
        Container? container = null;

        if (platformContainerCache.TryGetCacheEntry(deployment.PlatformId, out var cacheEntry, out _))
        {
            platformCache = cacheEntry;
            container = await unitOfWork.Containers.GetByDeploymentIdAsync(deploymentId, cancellationToken);
        }
        else
        {
            warnings.Add("The deployment platform is offline or unavailable. Deployment volume settings are used as a fallback.");
        }

        var volumeMetadata = StackComposeParser.ParseVolumeReferences(deployment.Spec?.Volumes);

        return Result.Success(new DeploymentBackupVolumeInput(
            deployment.Id,
            deployment.Name,
            deployment.PlatformId,
            platform?.Name ?? "Unknown platform",
            platform?.Status ?? PlatformStatus.Offline,
            platformCache,
            container,
            volumeMetadata.ServiceVolumeReferences.ToHashSet(StringComparer.Ordinal),
            volumeMetadata.HasAnonymousVolumes,
            warnings));
    }

    private static void AddVolume(
        IDictionary<string, ResolvedStackBackupVolume> volumes,
        Guid platformId,
        string volumeName,
        StackVolumeKind kind)
    {
        if (volumes.ContainsKey(volumeName))
            return;

        volumes[volumeName] = new ResolvedStackBackupVolume(
            platformId,
            volumeName,
            kind,
            IsExternal: false,
            IsShared: false);
    }

    private sealed record DeploymentBackupVolumeInput(
        Guid DeploymentId,
        string DeploymentName,
        Guid PlatformId,
        string PlatformName,
        PlatformStatus PlatformStatus,
        PlatformCacheEntry? PlatformCache,
        Container? Container,
        IReadOnlySet<string> DeclaredVolumeReferences,
        bool HasAnonymousVolumes,
        IReadOnlyList<string> Warnings);
}
