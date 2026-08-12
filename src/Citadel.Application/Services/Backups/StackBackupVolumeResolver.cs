using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Backups;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Backups;

internal sealed class StackBackupVolumeResolver(
    IServiceScopeFactory scopeFactory,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    ISwarmWorkloadBackupVolumeResolver swarmVolumeResolver)
    : IStackBackupVolumeResolver
{
    public async Task<Result<StackBackupVolumeResolution>> ResolveAsync(
        Guid stackId,
        CancellationToken cancellationToken)
    {
        var inputResult = await LoadInputAsync(stackId, cancellationToken);
        if (!inputResult.IsSuccess(out var input, out var inputError))
            return Result.Failure<StackBackupVolumeResolution>(inputError);

        var warnings = new List<string>(input.Warnings);
        if (input.PlatformEntity?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            var swarmVolumes = await swarmVolumeResolver.ResolveAsync(
                input.PlatformEntity,
                input.SwarmServices,
                cancellationToken);
            if (!swarmVolumes.IsSuccess(out var resolvedSwarmVolumes, out var swarmError))
                return Result.Failure<StackBackupVolumeResolution>(swarmError);

            foreach (var volume in resolvedSwarmVolumes.Where(static value => value.IsShared))
            {
                warnings.Add(
                    $"Volume {volume.VolumeName} on Node {volume.NodeHostname ?? volume.DockerNodeId} is mounted by more than one current Task.");
            }

            if (resolvedSwarmVolumes.Count == 0)
                warnings.Add("No supported Docker named volumes were resolved for this Swarm Stack.");

            return Result.Success(new StackBackupVolumeResolution(
                input.StackId,
                input.StackName,
                input.PlatformId,
                input.PlatformName,
                input.PlatformStatus,
                resolvedSwarmVolumes,
                [.. warnings.Distinct(StringComparer.Ordinal)]));
        }

        var volumes = new Dictionary<string, ResolvedStackBackupVolume>(StringComparer.Ordinal);
        var mountCounts = new Dictionary<string, int>(StringComparer.Ordinal);

        foreach (var binding in input.PersistedVolumeBindings)
        {
            AddVolume(volumes, binding);
        }

        if (volumes.Count == 0 && input.Platform is not null)
        {
            if (input.Containers.Count == 0)
            {
                warnings.Add("No synchronized containers were found for this stack. Compose declarations are used as a fallback.");
            }
            else
            {
                var connector = containerConnectorFactory.GetConnector(input.Platform.ConnectorType);
                foreach (var container in input.Containers)
                {
                    var inspect = await connector.InspectAsync(
                        new InspectContainerCommand(input.Platform.Address, container.DockerContainerId),
                        cancellationToken);

                    if (!inspect.IsSuccess(out var info, out var inspectError))
                    {
                        warnings.Add($"Could not inspect container {container.Name}: {inspectError.Message}");
                        continue;
                    }

                    foreach (var mount in info.Mounts)
                    {
                        if (!string.Equals(mount.Type, "volume", StringComparison.OrdinalIgnoreCase)
                            || string.IsNullOrWhiteSpace(mount.Name))
                        {
                            continue;
                        }

                        AddVolume(volumes, input.Compose, input.PlatformId, mount.Name, fromLiveMount: true);
                        mountCounts[mount.Name] = mountCounts.TryGetValue(mount.Name, out var count) ? count + 1 : 1;
                    }
                }
            }
        }
        else if (volumes.Count == 0)
        {
            warnings.Add("The stack platform is offline or unavailable. Compose declarations are used as a fallback.");
        }

        if (volumes.Count == 0)
        {
            foreach (var volumeName in input.Compose.ServiceVolumeReferences)
                AddVolume(volumes, input.Compose, input.PlatformId, volumeName, fromLiveMount: false);
        }

        if (input.Compose.HasAnonymousVolumes && volumes.Values.All(static volume => volume.Kind != StackVolumeKind.AnonymousNamed))
        {
            warnings.Add("The Compose file declares anonymous volumes. Their runtime names can only be resolved after stack containers are synchronized and inspectable.");
        }

        var resolved = volumes.Values
            .Select(volume =>
            {
                var isShared = mountCounts.TryGetValue(volume.VolumeName, out var count) && count > 1;
                return volume with { IsShared = isShared };
            })
            .OrderBy(static volume => volume.VolumeName, StringComparer.Ordinal)
            .ToArray();

        foreach (var volume in resolved)
        {
            if (volume.IsExternal)
                warnings.Add($"Volume {volume.VolumeName} is external to the stack and will be included.");

            if (volume.IsShared)
                warnings.Add($"Volume {volume.VolumeName} is mounted by more than one stack container.");
        }

        if (resolved.Length == 0)
            warnings.Add("No Docker named volumes were resolved for this stack.");

        return Result.Success(new StackBackupVolumeResolution(
            StackId: input.StackId,
            StackName: input.StackName,
            PlatformId: input.PlatformId,
            PlatformName: input.PlatformName,
            PlatformStatus: input.PlatformStatus,
            Volumes: resolved,
            Warnings: [.. warnings.Distinct(StringComparer.Ordinal)]));
    }

    private async Task<Result<StackBackupVolumeInput>> LoadInputAsync(
        Guid stackId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var stack = await unitOfWork.Stacks.GetAsync(stackId, cancellationToken);
        if (stack?.CurrentStackRelease is null)
            return Result.Failure<StackBackupVolumeInput>(new NotFoundError("Stack not found."));

        var release = stack.CurrentStackRelease;
        var compose = GetComposeVolumeMetadata(release.Spec);
        var warnings = new List<string>();
        PlatformCacheEntry? platform = null;
        var platformEntity = await unitOfWork.Platforms.GetByIdAsync(release.PlatformId, cancellationToken);
        IReadOnlyList<Container> containers = [];
        IReadOnlyList<SwarmServiceProjection> swarmServices = [];
        var bindings = await unitOfWork.Stacks.GetReleaseVolumeBindingsAsync(release.Id, cancellationToken);

        if (platformEntity?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            swarmServices = (await unitOfWork.Swarm.GetServicesAsync(release.PlatformId, cancellationToken))
                .Where(service => service.StackId == stack.Id)
                .ToArray();
        }
        else if (platformContainerCache.TryGetCacheEntry(release.PlatformId, out var platformEntry, out _))
        {
            platform = platformEntry;
            if (bindings.Count == 0)
                containers = (await unitOfWork.Stacks.GetContainersAsync(stackId, cancellationToken)).ToArray();
        }
        else
        {
            warnings.Add("The stack platform is offline or unavailable. Compose declarations are used as a fallback.");
        }

        return Result.Success(new StackBackupVolumeInput(
            stack.Id,
            stack.Name,
            release.PlatformId,
            release.Platform?.Name ?? "Unknown platform",
            release.Platform?.Status ?? PlatformStatus.Offline,
            compose,
            platform,
            containers,
            bindings,
            platformEntity,
            swarmServices,
            warnings));
    }

    private static void AddVolume(
        IDictionary<string, ResolvedStackBackupVolume> volumes,
        StackComposeVolumeMetadata compose,
        Guid platformId,
        string volumeName,
        bool fromLiveMount)
    {
        if (volumes.ContainsKey(volumeName))
            return;

        var isExternal = compose.ExternalVolumes.Contains(volumeName);
        var kind = isExternal
            ? StackVolumeKind.ExternalNamed
            : compose.DeclaredVolumes.Contains(volumeName)
                ? StackVolumeKind.DeclaredNamed
                : fromLiveMount
                    ? StackVolumeKind.AnonymousNamed
                    : StackVolumeKind.DeclaredNamed;

        volumes[volumeName] = new ResolvedStackBackupVolume(
            platformId,
            volumeName,
            kind,
            isExternal,
            IsShared: false);
    }

    private static void AddVolume(
        IDictionary<string, ResolvedStackBackupVolume> volumes,
        StackReleaseVolumeBinding binding)
    {
        if (volumes.ContainsKey(binding.VolumeName))
            return;

        var kind = binding.IsExternal
            ? StackVolumeKind.ExternalNamed
            : binding.IsAnonymous
                ? StackVolumeKind.AnonymousNamed
                : StackVolumeKind.DeclaredNamed;

        volumes[binding.VolumeName] = new ResolvedStackBackupVolume(
            binding.PlatformId,
            binding.VolumeName,
            kind,
            binding.IsExternal,
            IsShared: false);
    }

    private static StackComposeVolumeMetadata GetComposeVolumeMetadata(StackSpec spec)
    {
        if (spec is not ManualStack manual)
            return StackComposeVolumeMetadata.Empty;

        var resolution = StackComposeParser.ParseVolumes(manual.ComposeFile);
        var declared = resolution.DeclaredVolumes.Select(static volume => volume.Name).ToHashSet(StringComparer.Ordinal);
        var external = resolution.DeclaredVolumes
            .Where(static volume => volume.IsExternal)
            .Select(static volume => volume.Name)
            .ToHashSet(StringComparer.Ordinal);

        return new StackComposeVolumeMetadata(
            declared,
            external,
            resolution.ServiceVolumeReferences,
            resolution.HasAnonymousVolumes);
    }

    private sealed record StackComposeVolumeMetadata(
        IReadOnlySet<string> DeclaredVolumes,
        IReadOnlySet<string> ExternalVolumes,
        IReadOnlyList<string> ServiceVolumeReferences,
        bool HasAnonymousVolumes)
    {
        public static StackComposeVolumeMetadata Empty { get; } = new(
            new HashSet<string>(StringComparer.Ordinal),
            new HashSet<string>(StringComparer.Ordinal),
            [],
            HasAnonymousVolumes: false);
    }

    private sealed record StackBackupVolumeInput(
        Guid StackId,
        string StackName,
        Guid PlatformId,
        string PlatformName,
        PlatformStatus PlatformStatus,
        StackComposeVolumeMetadata Compose,
        PlatformCacheEntry? Platform,
        IReadOnlyList<Container> Containers,
        IReadOnlyList<StackReleaseVolumeBinding> PersistedVolumeBindings,
        Domain.Entities.Platforms.Platform? PlatformEntity,
        IReadOnlyList<SwarmServiceProjection> SwarmServices,
        IReadOnlyList<string> Warnings);
}
