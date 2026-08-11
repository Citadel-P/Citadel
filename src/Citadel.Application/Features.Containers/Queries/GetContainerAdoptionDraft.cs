using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using System.Text;

namespace Application.Features.Containers.Queries;

public enum AdoptionIssueSeverity
{
    Warning,
    Blocker
}

public sealed record AdoptionIssue(
    string Code,
    string Message,
    AdoptionIssueSeverity Severity,
    string? FieldPath = null);

public sealed record ContainerAdoptionSource(
    Guid Id,
    string DockerContainerId,
    string Name,
    Guid PlatformId,
    string PlatformName,
    ContainerStateStatus State);

public sealed record ContainerAdoptionDeploymentDraft(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    IReadOnlyCollection<Guid> TagIds);

public sealed record ContainerAdoptionDraft(
    ContainerAdoptionSource Source,
    ContainerAdoptionDeploymentDraft Draft,
    IReadOnlyCollection<AdoptionIssue> Issues,
    string PreviewFingerprint,
    bool CanImportSensitiveEnvironmentValues);

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record GetContainerAdoptionDraft(Guid ContainerId) : IQuery<Result<ContainerAdoptionDraft>>;

internal sealed class GetContainerAdoptionDraftHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IAdoptionFingerprintService fingerprintService)
    : IQueryHandler<GetContainerAdoptionDraft, Result<ContainerAdoptionDraft>>
{
    public async ValueTask<Result<ContainerAdoptionDraft>> Handle(
        GetContainerAdoptionDraft query,
        CancellationToken cancellationToken)
    {
        var contextResult = await ContainerAdoptionDraftFactory.LoadContextAsync(
            query.ContainerId,
            unitOfWork,
            connectorFactory,
            imageConnectorFactory,
            containerAuthorizationService,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<ContainerAdoptionDraft>(contextResult.Errors);

        var name = await GetAvailableNameAsync(context.Container.Name, context.Container.PlatformId, cancellationToken);
        return ContainerAdoptionDraftFactory.Create(context, name, fingerprintService);
    }

    private async Task<string> GetAvailableNameAsync(
        string containerName,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var baseName = ContainerAdoptionDraftFactory.NormalizeName(containerName);
        if (!await unitOfWork.Deployments.ExistsAsync(baseName, platformId, cancellationToken))
            return baseName;

        for (var suffix = 2; suffix <= 100; suffix++)
        {
            var suffixText = $"-{suffix}";
            var candidate = $"{baseName[..Math.Min(baseName.Length, 64 - suffixText.Length)].TrimEnd('-', '_')}{suffixText}";
            if (!await unitOfWork.Deployments.ExistsAsync(candidate, platformId, cancellationToken))
                return candidate;
        }

        return $"{baseName[..Math.Min(baseName.Length, 55)].TrimEnd('-', '_')}-{Guid.NewGuid():N}"[..64];
    }
}

internal sealed record ContainerAdoptionContext(
    Container Container,
    Platform Platform,
    ContainerInspectionInfo Inspection,
    Image? Image,
    InspectImageResult? ImageInspection);

internal static class ContainerAdoptionDraftFactory
{
    internal static async Task<Result<ContainerAdoptionContext>> LoadContextAsync(
        Guid containerId,
        IUnitOfWork unitOfWork,
        IConnectorFactory<IContainerConnector> connectorFactory,
        IConnectorFactory<IImageConnector> imageConnectorFactory,
        IContainerAuthorizationService containerAuthorizationService,
        CancellationToken cancellationToken)
    {
        var container = await unitOfWork.Containers.GetByIdAsync(containerId, cancellationToken);
        if (container is null)
            return Result.Failure<ContainerAdoptionContext>(new NotFoundError("Container does not exist."));

        var hasAccess = await containerAuthorizationService.HasAccessAsync(
            [container.DockerContainerId],
            ResourceType.Platform,
            PermissionLevel.Read,
            SpecificPermission.Inspect,
            cancellationToken);
        if (!hasAccess)
            return Result.Failure<ContainerAdoptionContext>(
                new ForbiddenError("Missing specific permission [Inspect] on [Platform]."));

        if (container.IsSystem)
            return Result.Failure<ContainerAdoptionContext>(new BadRequestError("System containers cannot be adopted."));
        if (container.IsSwarmTask)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new BadRequestError("Docker Swarm task containers cannot be adopted. Import their Stack instead."));
        }

        if (container.DeploymentId is not null || container.StackId is not null)
            return Result.Failure<ContainerAdoptionContext>(new ConflictError("Container is already managed by Citadel."));
        if (container.ControlState == ResourceControlState.Processing)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new ConflictError("Container is currently processing another operation."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(container.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<ContainerAdoptionContext>(new NotFoundError("Container platform does not exist."));

        if (platform.PlatformDescriptor.Type != PlatformType.Docker)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new BadRequestError("Only containers on Docker Standalone platforms can be adopted as deployments."));
        }

        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<ContainerAdoptionContext>(new ConflictError("Container platform is offline."));

        if (string.IsNullOrWhiteSpace(container.DockerImageId))
            return Result.Failure<ContainerAdoptionContext>(new ConflictError("Container image is not available."));

        var image = container.ImageId is null
            ? await unitOfWork.Images.GetByDockerImageIdAsync(
                container.DockerImageId,
                container.PlatformId,
                cancellationToken)
            : await unitOfWork.Images.GetByIdAsync(container.ImageId.Value, container.PlatformId, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        var inspectionResult = await connectorFactory
            .GetConnector(platform.ConnectorType)
            .InspectAsync(new InspectContainerCommand(platform.Address, container.DockerContainerId), cancellationToken);
        if (!inspectionResult.IsSuccess(out var inspection))
            return Result.Failure<ContainerAdoptionContext>(inspectionResult.Errors);

        if (!string.Equals(inspection.Id, container.DockerContainerId, StringComparison.OrdinalIgnoreCase))
            return Result.Failure<ContainerAdoptionContext>(
                new ConflictError("Container identity changed while preparing the adoption draft."));

        InspectImageResult? imageInspection = null;
        if (image is not null && RequiresImageDefaultComparison(inspection.Config))
        {
            var imageInspectionResult = await imageConnectorFactory
                .GetConnector(platform.ConnectorType)
                .InspectImageAsync(
                    new InspectImageCommand(platform.Address, image.DockerImageId),
                    cancellationToken);
            if (imageInspectionResult.IsSuccess(out var inspectedImage)
                && string.Equals(inspectedImage.Id, image.DockerImageId, StringComparison.OrdinalIgnoreCase))
            {
                imageInspection = inspectedImage;
            }
        }

        return new ContainerAdoptionContext(container, platform, inspection, image, imageInspection);
    }

    internal static ContainerAdoptionDraft Create(
        ContainerAdoptionContext context,
        string name,
        IAdoptionFingerprintService fingerprintService)
    {
        var issues = new List<AdoptionIssue>();
        var inspection = context.Inspection;
        var config = inspection.Config;
        var host = inspection.HostConfig;

        AddEligibilityIssues(
            config,
            host,
            context.ImageInspection,
            deferImageDefaultValidation: context.Image is null,
            issues);
        if (context.Image is null)
        {
            issues.Add(new AdoptionIssue(
                "SOURCE_IMAGE_UNAVAILABLE",
                "The container's original Docker image is no longer available. Select the intended local replacement or external image; Citadel verifies the repository when Docker still exposes the original reference. Future Apply operations will use the selected image.",
                AdoptionIssueSeverity.Warning,
                "spec.image.imageId"));
        }

        var sensitiveNames = GetSensitiveEnvironmentNames(inspection);
        var importableSensitiveValues = GetImportableSensitiveEnvironmentValues(inspection);
        var canImportSensitiveEnvironmentValues = sensitiveNames.Count > 0
                                                   && sensitiveNames.All(
                                                       name => IsValidEnvironmentBindingName(name)
                                                               && importableSensitiveValues.ContainsKey(name));

        var spec = new DeploymentSpec(
            Image: new LocalImage(context.Image?.Id.ToString() ?? string.Empty),
            UpdateBehavior: UpdateBehavior.Disabled,
            LifeCycleSpec: new LifeCycleSpec(
                StopTimeout: config?.StopTimeout,
                StopSignal: MapStopSignal(config?.StopSignal),
                RestartPolicy: MapRestartPolicy(host?.RestartPolicy?.Name) ?? ContainerRestartPolicy.No),
            ResourceSpec: new ResourceSpec(
                NanoCpus: host?.NanoCpus is > 0 ? (float)(host.NanoCpus.Value / 1_000_000_000d) : null,
                MemoryLimit: host?.Memory is > 0 ? (float)(host.Memory.Value / 1024d / 1024d) : null),
            Labels: MapLabels(config?.Labels, issues),
            Ports: MapPorts(host?.PortBindings, issues),
            Volumes: MapVolumes(inspection.Mounts, issues),
            Networks: inspection.NetworkSettings?.Networks.Keys.Order(StringComparer.Ordinal).ToList() ?? [],
            Command: config?.Cmd.ToList() ?? [],
            EnvironmentVariables: MapEnvironment(config?.Env, issues, canImportSensitiveEnvironmentValues));

        return new ContainerAdoptionDraft(
            new ContainerAdoptionSource(
                context.Container.Id,
                context.Container.DockerContainerId,
                context.Container.Name,
                context.Platform.Id,
                context.Platform.Name,
                context.Container.State),
            new ContainerAdoptionDeploymentDraft(
                name,
                context.Platform.Id,
                $"Adopted from Docker container {context.Container.Name}.",
                spec,
                []),
            issues,
            ComputeFingerprint(context, fingerprintService),
            canImportSensitiveEnvironmentValues);
    }

    internal static string ComputeFingerprint(
        ContainerAdoptionContext context,
        IAdoptionFingerprintService fingerprintService)
        => fingerprintService.Sign(
            $"{ComputeFingerprint(context.Container, context.Inspection, fingerprintService)}"
            + $"|{context.Image?.Id.ToString("N")}|{context.Image?.Name}");

    internal static string ComputeFingerprint(
        Container container,
        ContainerInspectionInfo inspection,
        IAdoptionFingerprintService fingerprintService)
    {
        var value = new StringBuilder(2048);
        Append(value, container.Id);
        Append(value, container.PlatformId);
        Append(value, container.DockerContainerId);
        Append(value, container.DockerImageId);
        Append(value, container.DeploymentId);
        Append(value, container.StackId);
        Append(value, container.IsSystem);
        Append(value, container.State);
        Append(value, container.ControlState);
        Append(value, inspection.Id);
        Append(value, inspection.Config?.Image);
        Append(value, inspection.Config?.User);
        Append(value, inspection.Config?.WorkingDir);
        AppendMany(value, inspection.Config?.Entrypoint);
        AppendMany(value, inspection.Config?.Cmd);
        AppendMany(value, inspection.Config?.Env);
        AppendDictionary(value, inspection.Config?.Labels);
        Append(value, inspection.HostConfig?.NetworkMode);
        Append(value, inspection.HostConfig?.RestartPolicy?.Name);
        Append(value, inspection.HostConfig?.RestartPolicy?.MaximumRetryCount);
        Append(value, inspection.HostConfig?.Privileged);
        Append(value, inspection.HostConfig?.ReadonlyRootfs);
        Append(value, inspection.HostConfig?.NanoCpus);
        Append(value, inspection.HostConfig?.Memory);
        AppendMany(value, inspection.HostConfig?.CapAdd);
        AppendMany(value, inspection.HostConfig?.CapDrop);
        AppendMany(value, inspection.HostConfig?.SecurityOpt);

        foreach (var mount in inspection.Mounts.OrderBy(x => x.Destination, StringComparer.Ordinal))
        {
            Append(value, mount.Type);
            Append(value, mount.Name);
            Append(value, mount.Source);
            Append(value, mount.Destination);
            Append(value, mount.Mode);
            Append(value, mount.RW);
            Append(value, mount.Propagation);
        }

        foreach (var port in (inspection.HostConfig?.PortBindings ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>())
                     .OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            Append(value, port.Key);
            foreach (var binding in port.Value.OrderBy(x => x.HostIP, StringComparer.Ordinal)
                         .ThenBy(x => x.HostPort, StringComparer.Ordinal))
            {
                Append(value, binding.HostIP);
                Append(value, binding.HostPort);
            }
        }

        foreach (var network in (inspection.NetworkSettings?.Networks ?? new Dictionary<string, EndpointSettingsInfo>())
                     .OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            Append(value, network.Key);
            Append(value, network.Value.NetworkID);
            Append(value, network.Value.IpAddress);
        }

        return fingerprintService.Sign(value.ToString());
    }

    internal static IReadOnlyList<string> GetSensitiveEnvironmentNames(ContainerInspectionInfo inspection)
        => (inspection.Config?.Env ?? [])
            .Select(GetEnvironmentName)
            .Where(ContainerInspectionRedactor.IsSensitiveEnvironmentName)
            .Distinct(StringComparer.Ordinal)
            .ToArray();

    internal static IReadOnlyDictionary<string, string> GetImportableSensitiveEnvironmentValues(
        ContainerInspectionInfo inspection)
    {
        var values = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var entry in inspection.Config?.Env ?? [])
        {
            var separator = entry.IndexOf('=');
            if (separator <= 0)
                continue;

            var name = entry[..separator].Trim();
            var value = entry[(separator + 1)..];
            if (!ContainerInspectionRedactor.IsSensitiveEnvironmentName(name)
                || string.IsNullOrEmpty(value)
                || string.Equals(value, ContainerInspectionRedactor.RedactedValue, StringComparison.Ordinal))
            {
                continue;
            }

            values[name] = value;
        }

        return values;
    }

    internal static bool IsValidEnvironmentBindingName(string name)
        => name.Length is > 0 and <= 128
           && (char.IsAsciiLetter(name[0]) || name[0] == '_')
           && name.Skip(1).All(character => char.IsAsciiLetterOrDigit(character) || character == '_');

    internal static List<string> UseSensitiveEnvironmentBindingReferences(
        IEnumerable<string>? environment,
        IReadOnlyCollection<string> sensitiveNames)
    {
        var names = sensitiveNames.ToHashSet(StringComparer.Ordinal);
        var referenced = new HashSet<string>(StringComparer.Ordinal);
        var result = new List<string>();

        foreach (var entry in environment ?? [])
        {
            var name = GetEnvironmentName(entry);
            if (!names.Contains(name))
            {
                result.Add(entry);
                continue;
            }

            if (referenced.Add(name))
                result.Add(BuildEnvironmentBindingReference(name));
        }

        foreach (var name in sensitiveNames)
        {
            if (referenced.Add(name))
                result.Add(BuildEnvironmentBindingReference(name));
        }

        return result;
    }

    internal static bool HasResolvedSensitiveEnvironment(
        IEnumerable<string>? environment,
        string name,
        IReadOnlySet<string> availableBindingNames)
    {
        var entry = environment?.FirstOrDefault(
            x => string.Equals(GetEnvironmentName(x), name, StringComparison.Ordinal));
        if (entry is null)
            return false;

        var separator = entry.IndexOf('=');
        if (separator < 0)
            return availableBindingNames.Contains(name);

        var value = entry[(separator + 1)..];
        if (string.IsNullOrWhiteSpace(value)
            || string.Equals(value, ContainerInspectionRedactor.RedactedValue, StringComparison.Ordinal))
        {
            return false;
        }

        foreach (var reference in GetEnvironmentReferences(value))
        {
            if (!availableBindingNames.Contains(reference))
                return false;
        }

        return true;
    }

    internal static string NormalizeName(string name)
    {
        var trimmed = name.Trim().TrimStart('/');
        var builder = new StringBuilder(Math.Min(trimmed.Length, 64));
        foreach (var character in trimmed)
        {
            if (builder.Length == 64)
                break;

            builder.Append(char.IsAsciiLetterOrDigit(character) || character is '-' or '_' ? character : '-');
        }

        var result = builder.ToString().Trim('-', '_');
        if (result.Length < 3)
            result = string.IsNullOrEmpty(result) ? "app" : $"{result}-app";

        return result[..Math.Min(result.Length, 64)];
    }

    private static Dictionary<string, string> MapLabels(
        IReadOnlyDictionary<string, string>? labels,
        ICollection<AdoptionIssue> issues)
    {
        var result = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var label in labels ?? new Dictionary<string, string>())
        {
            if (label.Key.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase)
                || label.Key.StartsWith(CitadelLabels.LegacyExtensionPrefix, StringComparison.OrdinalIgnoreCase))
            {
                issues.Add(new AdoptionIssue(
                    "CITADEL_OWNERSHIP_LABEL",
                    $"The container has Citadel ownership label '{label.Key}'.",
                    AdoptionIssueSeverity.Blocker,
                    "spec.labels"));
                continue;
            }

            if (label.Key.StartsWith("com.docker.compose.", StringComparison.OrdinalIgnoreCase))
                continue;

            result[label.Key] = label.Value;
        }

        return result;
    }

    private static List<string> MapEnvironment(
        IReadOnlyList<string>? environment,
        ICollection<AdoptionIssue> issues,
        bool useSensitiveBindingReferences)
    {
        var result = new List<string>(environment?.Count ?? 0);
        foreach (var entry in environment ?? [])
        {
            var name = GetEnvironmentName(entry);
            if (ContainerInspectionRedactor.IsSensitiveEnvironmentName(name))
            {
                result.Add(useSensitiveBindingReferences ? BuildEnvironmentBindingReference(name) : $"{name}=");
                issues.Add(new AdoptionIssue(
                    "SENSITIVE_ENVIRONMENT_VALUE_REQUIRED",
                    $"Enter a value or binding for sensitive environment variable '{name}'.",
                    AdoptionIssueSeverity.Warning,
                    $"spec.environmentVariables.{name}"));
                continue;
            }

            result.Add(entry);
        }

        return result;
    }

    private static List<string> MapPorts(
        IDictionary<string, IReadOnlyList<HostPortBinding>>? bindings,
        ICollection<AdoptionIssue> issues)
    {
        var result = new List<string>();
        foreach (var port in bindings ?? new Dictionary<string, IReadOnlyList<HostPortBinding>>())
        {
            var portParts = port.Key.Split('/', 2);
            if (!int.TryParse(portParts[0], out var containerPort))
            {
                issues.Add(new AdoptionIssue(
                    "UNSUPPORTED_PORT",
                    $"Port mapping '{port.Key}' could not be imported.",
                    AdoptionIssueSeverity.Warning,
                    "spec.ports"));
                continue;
            }

            var protocol = portParts.Length == 2 ? portParts[1] : "tcp";
            var hostBindings = port.Value.Where(x => !string.IsNullOrWhiteSpace(x.HostPort)).ToArray();
            if (hostBindings.Length == 0)
            {
                result.Add($"{containerPort}/{protocol}");
                continue;
            }

            foreach (var binding in hostBindings)
            {
                if (!int.TryParse(binding.HostPort, out var hostPort))
                    continue;

                if (!string.IsNullOrWhiteSpace(binding.HostIP)
                    && binding.HostIP is not "0.0.0.0" and not "::")
                {
                    issues.Add(new AdoptionIssue(
                        "HOST_IP_NOT_PRESERVED",
                        $"Host IP '{binding.HostIP}' for port {hostPort} is not represented by deployments.",
                        AdoptionIssueSeverity.Warning,
                        "spec.ports"));
                }

                result.Add($"{hostPort}:{containerPort}/{protocol}");
            }
        }

        return result;
    }

    private static List<string> MapVolumes(
        IReadOnlyList<MountPointInfo> mounts,
        ICollection<AdoptionIssue> issues)
    {
        var result = new List<string>(mounts.Count);
        foreach (var mount in mounts)
        {
            if (string.IsNullOrWhiteSpace(mount.Destination))
                continue;

            if (string.Equals(mount.Type, "tmpfs", StringComparison.OrdinalIgnoreCase))
            {
                issues.Add(new AdoptionIssue(
                    "TMPFS_NOT_SUPPORTED",
                    $"Tmpfs mount '{mount.Destination}' is not supported by deployments.",
                    AdoptionIssueSeverity.Blocker,
                    "spec.volumes"));
                continue;
            }

            var source = string.Equals(mount.Type, "volume", StringComparison.OrdinalIgnoreCase)
                ? mount.Name
                : mount.Source;
            if (string.IsNullOrWhiteSpace(source))
                continue;

            result.Add($"{source}:{mount.Destination}{(mount.RW == false ? ":ro" : string.Empty)}");

            if (!string.IsNullOrWhiteSpace(mount.Propagation)
                && !string.Equals(mount.Propagation, "rprivate", StringComparison.OrdinalIgnoreCase))
            {
                issues.Add(new AdoptionIssue(
                    "MOUNT_PROPAGATION_NOT_PRESERVED",
                    $"Mount propagation '{mount.Propagation}' is not represented by deployments.",
                    AdoptionIssueSeverity.Blocker,
                    "spec.volumes"));
            }
        }

        return result;
    }

    private static void AddEligibilityIssues(
        ContainerConfiguration? config,
        HostConfiguration? host,
        InspectImageResult? imageInspection,
        bool deferImageDefaultValidation,
        ICollection<AdoptionIssue> issues)
    {
        if (config?.Labels.TryGetValue(ComposeLabels.Project, out var project) == true
            && !string.IsNullOrWhiteSpace(project))
        {
            issues.Add(new AdoptionIssue(
                "COMPOSE_PROJECT_CONTAINER",
                $"This container belongs to Compose project '{project}'. Import the project as a stack instead.",
                AdoptionIssueSeverity.Blocker));
        }

        AddBlocker(host?.Privileged == true, "PRIVILEGED_CONTAINER", "Privileged containers cannot be adopted safely.");
        AddBlocker(host?.ReadonlyRootfs == true, "READ_ONLY_ROOT_FILESYSTEM", "Read-only root filesystems are not represented by deployments.");
        AddBlocker(host?.CapAdd.Count > 0 || host?.CapDrop.Count > 0, "CUSTOM_CAPABILITIES", "Custom Linux capabilities are not represented by deployments.");
        AddBlocker(host?.SecurityOpt.Count > 0, "SECURITY_OPTIONS", "Custom security options are not represented by deployments.");
        AddBlocker(host?.Tmpfs.Count > 0, "TMPFS_NOT_SUPPORTED", "Tmpfs mounts are not represented by deployments.");
        AddBlocker(host?.GroupAdd.Count > 0, "SUPPLEMENTARY_GROUPS", "Supplementary groups are not represented by deployments.");
        AddBlocker(host?.Sysctls.Count > 0, "CUSTOM_SYSCTLS", "Custom sysctls are not represented by deployments.");
        AddBlocker(host?.StorageOpt.Count > 0, "STORAGE_OPTIONS", "Custom storage options are not represented by deployments.");
        AddBlocker(host?.VolumesFrom.Count > 0, "VOLUMES_FROM", "Volumes inherited from another container are not represented by deployments.");
        AddBlocker(host?.Links.Count > 0, "CONTAINER_LINKS", "Legacy container links are not represented by deployments.");
        AddBlocker(host?.PublishAllPorts == true, "PUBLISH_ALL_PORTS", "Publishing all exposed ports is not represented by deployments.");
        AddBlocker(!string.IsNullOrWhiteSpace(host?.ContainerIDFile), "CONTAINER_ID_FILE", "Container ID files are not represented by deployments.");
        AddBlocker(!string.IsNullOrWhiteSpace(host?.PidMode), "PID_MODE", "Custom PID namespace mode is not represented by deployments.");
        AddBlocker(
            !string.IsNullOrWhiteSpace(host?.IpcMode)
            && !string.Equals(host.IpcMode, "private", StringComparison.OrdinalIgnoreCase),
            "IPC_MODE",
            "Custom IPC namespace mode is not represented by deployments.");
        AddBlocker(
            host?.NetworkMode is "host" or "none" || host?.NetworkMode?.StartsWith("container:", StringComparison.OrdinalIgnoreCase) == true,
            "NETWORK_MODE",
            $"Network mode '{host?.NetworkMode}' is not represented by deployments.");
        AddBlocker(
            !IsSupportedCgroupNamespaceMode(host?.CgroupnsMode),
            "CGROUP_NAMESPACE_MODE",
            "Custom cgroup namespace mode is not represented by deployments.");
        AddBlocker(!string.IsNullOrWhiteSpace(host?.UtsMode), "UTS_MODE", "Custom UTS namespace mode is not represented by deployments.");
        AddBlocker(!string.IsNullOrWhiteSpace(host?.UsernsMode), "USER_NAMESPACE_MODE", "Custom user namespace mode is not represented by deployments.");
        AddBlocker(!string.IsNullOrWhiteSpace(host?.Cgroup), "CUSTOM_CGROUP", "Custom cgroup placement is not represented by deployments.");
        AddBlocker(
            !string.IsNullOrWhiteSpace(host?.Runtime)
            && !string.Equals(host.Runtime, "runc", StringComparison.OrdinalIgnoreCase),
            "CUSTOM_RUNTIME",
            "Custom container runtimes are not represented by deployments.");
        AddBlocker(
            !string.IsNullOrWhiteSpace(host?.Isolation)
            && !string.Equals(host.Isolation, "default", StringComparison.OrdinalIgnoreCase),
            "CUSTOM_ISOLATION",
            "Custom container isolation is not represented by deployments.");
        AddBlocker(
            host?.ShmSize is > 0 and not 67_108_864,
            "SHARED_MEMORY_SIZE",
            "Custom shared-memory size is not represented by deployments.");
        AddBlocker(
            host?.PidsLimit is > 0,
            "PID_LIMIT",
            "PID limits are not represented by deployments.");
        AddBlocker(
            IsUnsupportedMemoryConfiguration(
                host?.Memory,
                host?.MemoryReservation,
                host?.MemorySwap,
                host?.MemorySwappiness),
            "MEMORY_CONFIGURATION",
            "Memory reservation, swap, or swappiness settings are not represented by deployments.");
        AddBlocker(
            host?.IoMaximumBandwidth is > 0
            || IsUnsupportedCpuPeriod(host?.CpuPeriod)
            || host?.CpuPercent is > 0
            || host?.CpuCount is > 0
            || host?.KernelMemoryTCP is > 0,
            "RESOURCE_LIMIT",
            "One or more resource limits are not represented by deployments.");
        AddBlocker(host?.OomScoreAdj is not null and not 0, "OOM_SCORE", "Custom OOM scoring is not represented by deployments.");
        AddBlocker(
            MapRestartPolicy(host?.RestartPolicy?.Name) == ContainerRestartPolicy.OnFailure
            && host?.RestartPolicy?.MaximumRetryCount is > 0,
            "RESTART_RETRY_LIMIT",
            "Restart retry limits are not represented by deployments.");
        AddBlocker(
            MapRestartPolicy(host?.RestartPolicy?.Name) is null,
            "RESTART_POLICY_NOT_SUPPORTED",
            $"Restart policy '{host?.RestartPolicy?.Name}' is not represented by deployments.");
        AddBlocker(
            !string.IsNullOrWhiteSpace(config?.StopSignal) && MapStopSignal(config.StopSignal) is null,
            "STOP_SIGNAL_NOT_SUPPORTED",
            $"Stop signal '{config?.StopSignal}' is not represented by deployments.");

        if (!deferImageDefaultValidation)
            AddImageOverrideBlockers(config, imageInspection, issues);
        AddBlocker(host?.AutoRemove == true, "AUTO_REMOVE_NOT_PRESERVED", "Auto-remove is not represented by deployments.");
        AddBlocker(host?.Dns.Count > 0 || host?.DnsOptions.Count > 0 || host?.DnsSearch.Count > 0, "DNS_NOT_PRESERVED", "Custom DNS settings are not represented by deployments.");
        AddBlocker(host?.ExtraHosts.Count > 0, "EXTRA_HOSTS_NOT_PRESERVED", "Extra host mappings are not represented by deployments.");
        AddBlocker(host?.Ulimits.Count > 0, "ULIMITS_NOT_PRESERVED", "Custom ulimits are not represented by deployments.");
        AddWarning(
            !IsLoggingConfigurationRepresented(host?.LogConfig),
            "LOGGING_NOT_PRESERVED",
            "The container's logging driver or logging options are not represented by deployments.");

        void AddBlocker(bool condition, string code, string message)
        {
            if (condition)
                issues.Add(new AdoptionIssue(code, message, AdoptionIssueSeverity.Blocker));
        }

        void AddWarning(bool condition, string code, string message)
        {
            if (condition)
                issues.Add(new AdoptionIssue(code, message, AdoptionIssueSeverity.Warning));
        }
    }

    private static void AddImageOverrideBlockers(
        ContainerConfiguration? config,
        InspectImageResult? imageInspection,
        ICollection<AdoptionIssue> issues)
    {
        if (!RequiresImageDefaultComparison(config))
            return;

        if (imageInspection is null)
        {
            issues.Add(new AdoptionIssue(
                "IMAGE_DEFAULTS_UNAVAILABLE",
                "Citadel could not verify whether process settings are inherited from the image.",
                AdoptionIssueSeverity.Blocker));
            return;
        }

        AddMismatch(
            !ImageUsersAreEquivalent(config?.User, imageInspection.User),
            "USER_NOT_PRESERVED",
            "The container user override differs from the selected image.");
        AddMismatch(
            !(config?.Entrypoint ?? []).SequenceEqual(imageInspection.EntryPoint ?? [], StringComparer.Ordinal),
            "ENTRYPOINT_NOT_PRESERVED",
            "The container entrypoint override differs from the selected image.");
        AddMismatch(
            !string.Equals(config?.WorkingDir ?? string.Empty, imageInspection.WorkingDir ?? string.Empty, StringComparison.Ordinal),
            "WORKING_DIRECTORY_NOT_PRESERVED",
            "The container working-directory override differs from the selected image.");

        void AddMismatch(bool condition, string code, string message)
        {
            if (condition)
                issues.Add(new AdoptionIssue(code, message, AdoptionIssueSeverity.Blocker));
        }
    }

    internal static bool IsLoggingConfigurationRepresented(LogConfiguration? configuration)
    {
        if (configuration is null)
            return true;

        if (configuration.Config.Count > 0)
            return false;

        return string.IsNullOrWhiteSpace(configuration.Type)
               || string.Equals(configuration.Type, "json-file", StringComparison.OrdinalIgnoreCase)
               || string.Equals(configuration.Type, "JsonFile", StringComparison.OrdinalIgnoreCase)
               || string.Equals(configuration.Type, "local", StringComparison.OrdinalIgnoreCase);
    }

    internal static bool RequiresImageDefaultComparison(ContainerConfiguration? config)
        => !string.IsNullOrWhiteSpace(config?.User)
           || config?.Entrypoint.Count > 0
           || !string.IsNullOrWhiteSpace(config?.WorkingDir);

    internal static bool IsUnsupportedCpuPeriod(long? cpuPeriod)
        => cpuPeriod is > 0 and not 100_000;

    internal static bool IsUnsupportedMemoryConfiguration(
        long? memory,
        long? memoryReservation,
        long? memorySwap,
        long? memorySwappiness)
    {
        if (memoryReservation is > 0 || memorySwappiness is >= 0)
            return true;

        if (memorySwap is null or 0)
            return false;

        return memorySwap == -1 || memory is not > 0 || memorySwap != memory * 2;
    }

    private static bool ImageUsersAreEquivalent(string? containerUser, string? imageUser)
        => string.Equals(
            NormalizeImageUser(containerUser),
            NormalizeImageUser(imageUser),
            StringComparison.Ordinal);

    private static string NormalizeImageUser(string? user)
    {
        var value = user?.Trim() ?? string.Empty;
        return value is "0" or "root" ? string.Empty : value;
    }

    internal static ContainerRestartPolicy? MapRestartPolicy(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)
            || value.Equals("no", StringComparison.OrdinalIgnoreCase)
            || value.Equals("empty", StringComparison.OrdinalIgnoreCase))
        {
            return ContainerRestartPolicy.No;
        }

        if (value.Equals("always", StringComparison.OrdinalIgnoreCase))
            return ContainerRestartPolicy.Always;

        if (value.Equals("on-failure", StringComparison.OrdinalIgnoreCase)
            || value.Equals("onfailure", StringComparison.OrdinalIgnoreCase))
        {
            return ContainerRestartPolicy.OnFailure;
        }

        if (value.Equals("unless-stopped", StringComparison.OrdinalIgnoreCase)
            || value.Equals("unlessstopped", StringComparison.OrdinalIgnoreCase))
        {
            return ContainerRestartPolicy.UnlessStopped;
        }

        return null;
    }

    internal static string BuildImportedSecretName(Guid deploymentId, string deploymentName, string environmentName)
    {
        const string prefix = "ADOPTED_";
        var suffix = $"_{deploymentId:N}".ToUpperInvariant();
        var source = $"{deploymentName}_{environmentName}".ToUpperInvariant();
        var normalized = new StringBuilder(source.Length);
        foreach (var character in source)
            normalized.Append(char.IsAsciiLetterOrDigit(character) || character == '_' ? character : '_');

        var body = normalized.ToString().Trim('_');
        var maxBodyLength = 128 - prefix.Length - suffix.Length;
        if (body.Length > maxBodyLength)
            body = body[..maxBodyLength];

        return $"{prefix}{body}{suffix}";
    }

    internal static bool IsSupportedCgroupNamespaceMode(string? value)
        => string.IsNullOrWhiteSpace(value)
           || value.Equals("private", StringComparison.OrdinalIgnoreCase)
           || value.Equals("host", StringComparison.OrdinalIgnoreCase);

    private static StopSignal? MapStopSignal(string? value)
        => Enum.TryParse<StopSignal>(value, ignoreCase: true, out var signal)
            ? signal
            : null;

    private static string GetEnvironmentName(string entry)
    {
        var separator = entry.IndexOf('=');
        return (separator < 0 ? entry : entry[..separator]).Trim();
    }

    private static string BuildEnvironmentBindingReference(string name)
        => $"{name}=${{{name}}}";

    private static IEnumerable<string> GetEnvironmentReferences(string value)
    {
        var searchIndex = 0;
        while (searchIndex < value.Length)
        {
            var start = value.IndexOf("${", searchIndex, StringComparison.Ordinal);
            if (start < 0)
                yield break;

            var end = value.IndexOf('}', start + 2);
            if (end < 0)
                yield break;

            var name = value[(start + 2)..end];
            if (!string.IsNullOrWhiteSpace(name))
                yield return name;

            searchIndex = end + 1;
        }
    }

    private static void Append(StringBuilder builder, object? value)
    {
        var text = value?.ToString() ?? string.Empty;
        builder.Append(text.Length).Append(':').Append(text).Append('|');
    }

    private static void AppendMany(StringBuilder builder, IEnumerable<string>? values)
    {
        foreach (var value in values ?? [])
            Append(builder, value);
    }

    private static void AppendDictionary(StringBuilder builder, IReadOnlyDictionary<string, string>? values)
    {
        foreach (var value in (values ?? new Dictionary<string, string>()).OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            Append(builder, value.Key);
            Append(builder, value.Value);
        }
    }

}
