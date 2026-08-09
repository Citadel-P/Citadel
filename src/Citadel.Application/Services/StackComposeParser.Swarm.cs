using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using System.Text;
using YamlDotNet.Core;
using YamlDotNet.RepresentationModel;

namespace Application.Services;

internal static partial class StackComposeParser
{
    private const int MaxSwarmComposeFiles = 16;
    private const int MaxSwarmComposeBytes = 4 * 1024 * 1024;

    private static readonly HashSet<string> SwarmTopLevelKeys = NewKeySet(
        "version", "services", "networks", "volumes", "secrets", "configs");

    private static readonly HashSet<string> SwarmServiceKeys = NewKeySet(
        "image", "build", "command", "entrypoint", "working_dir", "user", "environment", "env_file",
        "labels", "healthcheck", "hostname", "stop_grace_period", "logging", "deploy", "endpoint_mode",
        "networks", "volumes", "ports", "secrets", "configs", "tmpfs");

    private static readonly HashSet<string> SwarmDeployKeys = NewKeySet(
        "mode", "replicas", "placement", "resources", "restart_policy", "update_config", "rollback_config", "labels");

    private static readonly HashSet<string> SwarmPlacementKeys = NewKeySet(
        "constraints", "preferences", "max_replicas_per_node");

    private static readonly HashSet<string> SwarmResourceKeys = NewKeySet("limits", "reservations");
    private static readonly HashSet<string> SwarmResourceLimitKeys = NewKeySet("cpus", "memory", "pids");
    private static readonly HashSet<string> SwarmRestartPolicyKeys = NewKeySet("condition", "delay", "max_attempts", "window");
    private static readonly HashSet<string> SwarmUpdateKeys = NewKeySet(
        "parallelism", "delay", "failure_action", "monitor", "max_failure_ratio", "order");
    private static readonly HashSet<string> SwarmHealthcheckKeys = NewKeySet(
        "test", "interval", "timeout", "retries", "start_period", "start_interval", "disable");
    private static readonly HashSet<string> SwarmLoggingKeys = NewKeySet("driver", "options");
    private static readonly HashSet<string> SwarmVolumeKeys = NewKeySet("driver", "driver_opts", "external", "name", "labels");
    private static readonly HashSet<string> SwarmNetworkKeys = NewKeySet(
        "driver", "driver_opts", "attachable", "external", "name", "labels", "internal", "enable_ipv4", "enable_ipv6");
    private static readonly HashSet<string> SwarmSecretConfigKeys = NewKeySet(
        "file", "external", "name", "labels", "driver", "template_driver");
    private static readonly HashSet<string> SwarmMountKeys = NewKeySet(
        "type", "source", "target", "read_only", "bind", "volume", "tmpfs", "consistency");
    private static readonly HashSet<string> SwarmBindKeys = NewKeySet("propagation", "create_host_path", "selinux");
    private static readonly HashSet<string> SwarmNamedVolumeMountKeys = NewKeySet("nocopy", "subpath");
    private static readonly HashSet<string> SwarmTmpfsKeys = NewKeySet("size", "mode");
    private static readonly HashSet<string> SwarmPortKeys = NewKeySet("name", "target", "published", "protocol", "app_protocol", "mode");
    private static readonly HashSet<string> SwarmSecretConfigReferenceKeys = NewKeySet("source", "target", "uid", "gid", "mode");

    public static SwarmStackCompatibilityReport AnalyzeSwarmCompatibility(
        IReadOnlyList<string> composeFiles,
        IReadOnlyList<StackBuildImageBinding>? buildBindings = null)
    {
        var issues = new List<SwarmStackCompatibilityIssue>();
        var services = new Dictionary<string, SwarmComposeServiceState>(StringComparer.OrdinalIgnoreCase);
        var boundBuildServices = (buildBindings ?? [])
            .Select(static binding => binding.ServiceName)
            .Where(static name => !string.IsNullOrWhiteSpace(name))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        if (composeFiles.Count == 0)
        {
            AddError(issues, "compose.empty", "At least one Compose file is required.", "services");
            return CreateReport(issues);
        }

        var inputLimitReport = GetSwarmInputLimitReport(composeFiles.Count, 0);
        if (inputLimitReport is not null)
            return inputLimitReport;

        var totalBytes = 0L;
        foreach (var composeFile in composeFiles)
        {
            totalBytes += Encoding.UTF8.GetByteCount(composeFile);
            inputLimitReport = GetSwarmInputLimitReport(composeFiles.Count, totalBytes);
            if (inputLimitReport is not null)
                return inputLimitReport;
        }

        for (var fileIndex = 0; fileIndex < composeFiles.Count; fileIndex++)
        {
            var filePath = composeFiles.Count == 1 ? string.Empty : $"files[{fileIndex}].";
            YamlMappingNode? root;
            try
            {
                using var reader = new StringReader(composeFiles[fileIndex]);
                var yaml = new YamlStream();
                yaml.Load(reader);
                root = yaml.Documents.FirstOrDefault()?.RootNode as YamlMappingNode;
            }
            catch (YamlException ex)
            {
                AddError(issues, "compose.invalid_yaml", $"Compose YAML is invalid: {ex.Message}", filePath.TrimEnd('.'));
                continue;
            }

            if (root is null)
            {
                AddError(issues, "compose.invalid_root", "Compose must contain a YAML mapping at its root.", filePath.TrimEnd('.'));
                continue;
            }

            ValidateKeys(root, SwarmTopLevelKeys, filePath.TrimEnd('.'), issues, allowExtensions: true);
            ValidateTopLevelDefinitions(root, "networks", SwarmNetworkKeys, filePath, issues);
            ValidateTopLevelDefinitions(root, "volumes", SwarmVolumeKeys, filePath, issues);
            ValidateTopLevelDefinitions(root, "secrets", SwarmSecretConfigKeys, filePath, issues);
            ValidateTopLevelDefinitions(root, "configs", SwarmSecretConfigKeys, filePath, issues);

            if (!TryGetNode(root, "services", out var rawServices))
                continue;
            if (rawServices is not YamlMappingNode serviceNodes)
            {
                AddError(issues, "compose.invalid_structure", "'services' must be a mapping.", $"{filePath}services");
                continue;
            }

            foreach (var (serviceKey, serviceValue) in serviceNodes.Children)
            {
                if (serviceKey is not YamlScalarNode { Value: { Length: > 0 } serviceName })
                    continue;

                var servicePath = $"{filePath}services.{serviceName}";
                if (serviceValue is not YamlMappingNode service)
                {
                    AddError(issues, "service.invalid", $"Service '{serviceName}' must be a mapping.", servicePath);
                    continue;
                }

                if (!services.TryGetValue(serviceName, out var state))
                {
                    state = new SwarmComposeServiceState();
                    services[serviceName] = state;
                }

                ValidateKeys(service, SwarmServiceKeys, servicePath, issues);
                state.HasImage |= !string.IsNullOrWhiteSpace(TryGetScalar(service, "image"));
                state.HasBuild |= HasKey(service, "build");

                ValidateReservedLabels(service, "labels", $"{servicePath}.labels", issues);
                ValidateScalarEnum(service, "endpoint_mode", ["vip", "dnsrr"], servicePath, issues);
                ValidateNestedMapping(service, "healthcheck", SwarmHealthcheckKeys, servicePath, issues);
                ValidateNestedMapping(service, "logging", SwarmLoggingKeys, servicePath, issues);
                ValidateServiceDeploy(service, servicePath, issues);
                ValidateServiceMounts(service, servicePath, issues);
                ValidateServicePorts(service, servicePath, issues);
                ValidateReferenceList(service, "secrets", servicePath, issues);
                ValidateReferenceList(service, "configs", servicePath, issues);
            }
        }

        if (services.Count == 0)
            AddError(issues, "services.empty", "Compose must define at least one Service.", "services");

        foreach (var (serviceName, state) in services)
        {
            var hasBuildBinding = boundBuildServices.Contains(serviceName);
            if (!state.HasImage && !hasBuildBinding)
            {
                AddError(
                    issues,
                    "service.image_required",
                    $"Service '{serviceName}' requires an image or a Citadel Build binding that produces a registry image.",
                    $"services.{serviceName}.image");
            }

            if (state.HasBuild && !hasBuildBinding)
            {
                AddError(
                    issues,
                    "service.build_unsupported",
                    $"Service '{serviceName}' uses build without a Citadel Build binding.",
                    $"services.{serviceName}.build");
            }
        }

        return CreateReport(issues);
    }

    public static SwarmExternalResourceReferences ParseSwarmExternalResources(
        IReadOnlyList<string> composeFiles)
    {
        var networks = new HashSet<string>(StringComparer.Ordinal);
        var secrets = new HashSet<string>(StringComparer.Ordinal);
        var configs = new HashSet<string>(StringComparer.Ordinal);
        var volumes = new HashSet<string>(StringComparer.Ordinal);

        foreach (var composeFile in composeFiles)
        {
            using var reader = new StringReader(composeFile);
            var yaml = new YamlStream();
            yaml.Load(reader);
            if (yaml.Documents.FirstOrDefault()?.RootNode is not YamlMappingNode root)
                continue;

            AddExternalDefinitions(root, "networks", networks);
            AddExternalDefinitions(root, "secrets", secrets);
            AddExternalDefinitions(root, "configs", configs);
            AddExternalDefinitions(root, "volumes", volumes);
        }

        return new SwarmExternalResourceReferences(
            networks,
            secrets,
            configs,
            volumes);
    }

    private static void AddExternalDefinitions(
        YamlMappingNode root,
        string section,
        HashSet<string> destination)
    {
        if (!TryGetNode(root, section, out var rawDefinitions)
            || rawDefinitions is not YamlMappingNode definitions)
        {
            return;
        }

        foreach (var (definitionKey, definitionValue) in definitions.Children)
        {
            if (definitionKey is not YamlScalarNode { Value: { Length: > 0 } composeName }
                || definitionValue is not YamlMappingNode definition
                || !IsTrue(TryGetScalar(definition, "external")))
            {
                continue;
            }

            destination.Add(TryGetScalar(definition, "name")?.Trim() ?? composeName);
        }
    }

    private static bool IsTrue(string? value)
        => bool.TryParse(value, out var result) && result;

    private static void ValidateServiceDeploy(
        YamlMappingNode service,
        string servicePath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(service, "deploy", out var rawDeploy))
            return;
        if (rawDeploy is not YamlMappingNode deploy)
        {
            AddError(issues, "compose.invalid_structure", "'deploy' must be a mapping.", $"{servicePath}.deploy");
            return;
        }

        var deployPath = $"{servicePath}.deploy";
        ValidateKeys(deploy, SwarmDeployKeys, deployPath, issues);
        ValidateScalarEnum(deploy, "mode", ["replicated", "global"], deployPath, issues);
        ValidateReservedLabels(deploy, "labels", $"{deployPath}.labels", issues);
        ValidateNestedMapping(deploy, "placement", SwarmPlacementKeys, deployPath, issues);
        ValidateNestedMapping(deploy, "restart_policy", SwarmRestartPolicyKeys, deployPath, issues);
        ValidateNestedMapping(deploy, "update_config", SwarmUpdateKeys, deployPath, issues);
        ValidateNestedMapping(deploy, "rollback_config", SwarmUpdateKeys, deployPath, issues);
        ValidateNestedEnum(deploy, "restart_policy", "condition", ["none", "on-failure", "any"], deployPath, issues);
        ValidateNestedEnum(deploy, "update_config", "failure_action", ["pause", "continue", "rollback"], deployPath, issues);
        ValidateNestedEnum(deploy, "update_config", "order", ["stop-first", "start-first"], deployPath, issues);
        ValidateNestedEnum(deploy, "rollback_config", "failure_action", ["pause", "continue"], deployPath, issues);
        ValidateNestedEnum(deploy, "rollback_config", "order", ["stop-first", "start-first"], deployPath, issues);

        if (TryGetMapping(deploy, "resources", out var resources))
        {
            ValidateKeys(resources, SwarmResourceKeys, $"{deployPath}.resources", issues);
            ValidateNestedMapping(resources, "limits", SwarmResourceLimitKeys, $"{deployPath}.resources", issues);
            ValidateNestedMapping(resources, "reservations", SwarmResourceLimitKeys, $"{deployPath}.resources", issues);
        }
    }

    private static void ValidateServiceMounts(
        YamlMappingNode service,
        string servicePath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(service, "volumes", out var rawVolumes))
            return;
        if (rawVolumes is not YamlSequenceNode volumes)
        {
            AddError(issues, "compose.invalid_structure", "'volumes' must be a list.", $"{servicePath}.volumes");
            return;
        }

        for (var index = 0; index < volumes.Children.Count; index++)
        {
            var value = volumes.Children[index];
            var path = $"{servicePath}.volumes[{index}]";
            if (value is YamlScalarNode scalar)
            {
                var source = scalar.Value?.Split(':', 2)[0];
                if (!string.IsNullOrWhiteSpace(source) && IsLikelyHostPath(source))
                    AddWarning(issues, "mount.bind_portability", "Bind mounts require the same host path on every eligible Swarm node.", path);
                continue;
            }

            if (value is not YamlMappingNode mount)
            {
                AddError(issues, "compose.invalid_structure", "A volume reference must be a string or mapping.", path);
                continue;
            }

            ValidateKeys(mount, SwarmMountKeys, path, issues);
            ValidateNestedMapping(mount, "bind", SwarmBindKeys, path, issues);
            ValidateNestedMapping(mount, "volume", SwarmNamedVolumeMountKeys, path, issues);
            ValidateNestedMapping(mount, "tmpfs", SwarmTmpfsKeys, path, issues);
            ValidateScalarEnum(mount, "type", ["volume", "bind", "tmpfs"], path, issues);
            if (string.Equals(TryGetScalar(mount, "type"), "bind", StringComparison.OrdinalIgnoreCase))
                AddWarning(issues, "mount.bind_portability", "Bind mounts require the same host path on every eligible Swarm node.", path);
        }
    }

    private static void ValidateServicePorts(
        YamlMappingNode service,
        string servicePath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(service, "ports", out var rawPorts))
            return;
        if (rawPorts is not YamlSequenceNode ports)
        {
            AddError(issues, "compose.invalid_structure", "'ports' must be a list.", $"{servicePath}.ports");
            return;
        }

        for (var index = 0; index < ports.Children.Count; index++)
        {
            if (ports.Children[index] is YamlScalarNode)
                continue;
            if (ports.Children[index] is not YamlMappingNode port)
            {
                AddError(
                    issues,
                    "compose.invalid_structure",
                    "A port reference must be a string or mapping.",
                    $"{servicePath}.ports[{index}]");
                continue;
            }

            var path = $"{servicePath}.ports[{index}]";
            ValidateKeys(port, SwarmPortKeys, path, issues);
            ValidateScalarEnum(port, "mode", ["ingress", "host"], path, issues);
            ValidateScalarEnum(port, "protocol", ["tcp", "udp", "sctp"], path, issues);
            if (string.Equals(TryGetScalar(port, "mode"), "host", StringComparison.OrdinalIgnoreCase)
                && !string.IsNullOrWhiteSpace(TryGetScalar(port, "published")))
            {
                AddWarning(issues, "port.host_mode_portability", "A fixed Host-mode published port can schedule only where that port is available.", path);
            }
        }
    }

    private static void ValidateReferenceList(
        YamlMappingNode service,
        string key,
        string servicePath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(service, key, out var rawReferences))
            return;
        if (rawReferences is not YamlSequenceNode references)
        {
            AddError(issues, "compose.invalid_structure", $"'{key}' must be a list.", $"{servicePath}.{key}");
            return;
        }

        for (var index = 0; index < references.Children.Count; index++)
        {
            if (references.Children[index] is YamlMappingNode reference)
                ValidateKeys(reference, SwarmSecretConfigReferenceKeys, $"{servicePath}.{key}[{index}]", issues);
            else if (references.Children[index] is not YamlScalarNode)
                AddError(
                    issues,
                    "compose.invalid_structure",
                    $"A {key.TrimEnd('s')} reference must be a string or mapping.",
                    $"{servicePath}.{key}[{index}]");
        }
    }

    private static void ValidateTopLevelDefinitions(
        YamlMappingNode root,
        string key,
        HashSet<string> allowedKeys,
        string filePath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(root, key, out var rawDefinitions))
            return;
        if (rawDefinitions is not YamlMappingNode definitions)
        {
            AddError(issues, "compose.invalid_structure", $"'{key}' must be a mapping.", $"{filePath}{key}");
            return;
        }

        foreach (var (definitionKey, definitionValue) in definitions.Children)
        {
            if (definitionKey is not YamlScalarNode { Value: { Length: > 0 } name })
            {
                continue;
            }

            var path = $"{filePath}{key}.{name}";
            if (definitionValue is YamlScalarNode { Style: ScalarStyle.Plain } scalar
                && string.IsNullOrEmpty(scalar.Value))
                continue;
            if (definitionValue is not YamlMappingNode definition)
            {
                AddError(issues, "compose.invalid_structure", $"'{path}' must be a mapping.", path);
                continue;
            }

            ValidateKeys(definition, allowedKeys, path, issues, allowExtensions: true);
            ValidateReservedLabels(definition, "labels", $"{path}.labels", issues);

            if (key == "volumes"
                && string.Equals(TryGetScalar(definition, "driver"), "local", StringComparison.OrdinalIgnoreCase))
            {
                AddWarning(issues, "volume.local_portability", $"Volume '{name}' uses the node-local driver and is not portable between nodes.", path);
            }
        }
    }

    private static void ValidateNestedMapping(
        YamlMappingNode parent,
        string key,
        HashSet<string> allowedKeys,
        string parentPath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(parent, key, out var rawValue))
            return;
        if (rawValue is not YamlMappingNode mapping)
        {
            AddError(issues, "compose.invalid_structure", $"'{key}' must be a mapping.", $"{parentPath}.{key}");
            return;
        }

        ValidateKeys(mapping, allowedKeys, $"{parentPath}.{key}", issues);
    }

    private static void ValidateKeys(
        YamlMappingNode mapping,
        HashSet<string> allowedKeys,
        string path,
        List<SwarmStackCompatibilityIssue> issues,
        bool allowExtensions = false)
    {
        foreach (var key in mapping.Children.Keys.OfType<YamlScalarNode>())
        {
            var value = key.Value;
            if (string.IsNullOrWhiteSpace(value)
                || allowedKeys.Contains(value)
                || (allowExtensions
                    && value.StartsWith("x-", StringComparison.OrdinalIgnoreCase)
                    && !value.StartsWith(CitadelLabels.LegacyExtensionPrefix, StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            var fieldPath = string.IsNullOrWhiteSpace(path) ? value : $"{path}.{value}";
            AddError(
                issues,
                "compose.unsupported_key",
                $"'{fieldPath}' is not supported by Swarm Stacks.",
                fieldPath);
        }
    }

    private static void ValidateReservedLabels(
        YamlMappingNode parent,
        string key,
        string path,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (!TryGetNode(parent, key, out var labels))
            return;

        IEnumerable<string?> names = labels switch
        {
            YamlMappingNode mapping => mapping.Children.Keys.OfType<YamlScalarNode>().Select(static label => label.Value),
            YamlSequenceNode sequence => sequence.Children.OfType<YamlScalarNode>()
                .Select(static label => label.Value?.Split('=', 2)[0]),
            _ => []
        };

        foreach (var name in names)
        {
            if (name is null
                || (!name.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase)
                    && !name.StartsWith(CitadelLabels.LegacyExtensionPrefix, StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            AddError(issues, "labels.reserved", $"Label '{name}' is reserved for Citadel ownership.", path);
        }
    }

    private static void ValidateNestedEnum(
        YamlMappingNode parent,
        string mappingKey,
        string valueKey,
        IReadOnlyList<string> allowedValues,
        string parentPath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        if (TryGetMapping(parent, mappingKey, out var mapping))
            ValidateScalarEnum(mapping, valueKey, allowedValues, $"{parentPath}.{mappingKey}", issues);
    }

    private static void ValidateScalarEnum(
        YamlMappingNode mapping,
        string key,
        IReadOnlyList<string> allowedValues,
        string parentPath,
        List<SwarmStackCompatibilityIssue> issues)
    {
        var value = TryGetScalar(mapping, key);
        if (string.IsNullOrWhiteSpace(value)
            || allowedValues.Contains(value, StringComparer.OrdinalIgnoreCase))
        {
            return;
        }

        AddError(
            issues,
            "compose.unsupported_value",
            $"'{parentPath}.{key}' has unsupported value '{value}'.",
            $"{parentPath}.{key}");
    }

    private static bool TryGetNode(YamlMappingNode mapping, string key, out YamlNode value)
    {
        foreach (var (childKey, childValue) in mapping.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase))
            {
                value = childValue;
                return true;
            }
        }

        value = null!;
        return false;
    }

    private static bool HasKey(YamlMappingNode mapping, string key)
        => TryGetNode(mapping, key, out _);

    private static HashSet<string> NewKeySet(params string[] keys)
        => new(keys, StringComparer.OrdinalIgnoreCase);

    internal static SwarmStackCompatibilityReport? GetSwarmInputLimitReport(int fileCount, long totalBytes)
    {
        var issues = new List<SwarmStackCompatibilityIssue>(1);
        if (fileCount > MaxSwarmComposeFiles)
        {
            AddError(
                issues,
                "compose.too_many_files",
                $"Swarm Stack preflight accepts at most {MaxSwarmComposeFiles} Compose files.",
                "files");
        }
        else if (totalBytes > MaxSwarmComposeBytes)
        {
            AddError(
                issues,
                "compose.too_large",
                "The combined Compose input exceeds the 4 MiB preflight limit.",
                "files");
        }

        return issues.Count == 0 ? null : CreateReport(issues);
    }

    private static SwarmStackCompatibilityReport CreateReport(List<SwarmStackCompatibilityIssue> issues)
        => new(
            IsCompatible: issues.All(static issue => issue.Severity != SwarmStackCompatibilitySeverity.Error),
            Issues: issues);

    private static void AddError(
        List<SwarmStackCompatibilityIssue> issues,
        string code,
        string message,
        string? path)
        => issues.Add(new(SwarmStackCompatibilitySeverity.Error, code, message, path));

    private static void AddWarning(
        List<SwarmStackCompatibilityIssue> issues,
        string code,
        string message,
        string? path)
        => issues.Add(new(SwarmStackCompatibilitySeverity.Warning, code, message, path));

    private sealed class SwarmComposeServiceState
    {
        public bool HasImage { get; set; }
        public bool HasBuild { get; set; }
    }
}

internal sealed record SwarmExternalResourceReferences(
    IReadOnlySet<string> Networks,
    IReadOnlySet<string> Secrets,
    IReadOnlySet<string> Configs,
    IReadOnlySet<string> Volumes)
{
    public bool IsEmpty => Networks.Count == 0 && Secrets.Count == 0 && Configs.Count == 0 && Volumes.Count == 0;
}
