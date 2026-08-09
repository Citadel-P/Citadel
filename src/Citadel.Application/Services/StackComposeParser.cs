using Domain.Entities.Stacks;
using YamlDotNet.RepresentationModel;

namespace Application.Services;

internal static partial class StackComposeParser
{
    public static IReadOnlyDictionary<string, StackComposeService> ParseServices(
        Guid stackId,
        Guid releaseId,
        string composeFile)
    {
        if (string.IsNullOrWhiteSpace(composeFile))
            return new Dictionary<string, StackComposeService>(StringComparer.OrdinalIgnoreCase);

        var managedComposeFile = StackComposeLabelInjector.Inject(composeFile, stackId, releaseId);
        return ParseServicesFromYaml(managedComposeFile);
    }

    public static IReadOnlyDictionary<string, StackComposeService> ParseServices(
        Guid stackId,
        Guid releaseId,
        IReadOnlyList<string> composeFiles)
    {
        var services = new Dictionary<string, StackComposeService>(StringComparer.OrdinalIgnoreCase);

        if (composeFiles.Count == 0)
            return services;

        foreach (var composeFile in composeFiles)
        {
            foreach (var service in ParseServicesFromYaml(composeFile).Values)
            {
                services[service.ServiceName] = service;
            }
        }

        var labelsOverride = StackComposeLabelInjector.CreateLabelsOverride(composeFiles, stackId, releaseId);
        foreach (var service in ParseServicesFromYaml(labelsOverride).Values)
        {
            if (services.TryGetValue(service.ServiceName, out var existing))
            {
                services[service.ServiceName] = existing with { ExpectedConfigHash = service.ExpectedConfigHash };
            }
        }

        return services;
    }

    public static IReadOnlyList<StackComposeEnvironmentBindingReference> ParseEnvironmentBindingReferences(
        IReadOnlyList<string> composeFiles)
    {
        var references = new Dictionary<string, Dictionary<string, string>>(StringComparer.OrdinalIgnoreCase);

        foreach (var composeFile in composeFiles)
        {
            if (string.IsNullOrWhiteSpace(composeFile))
                continue;

            using var reader = new StringReader(composeFile);
            var yaml = new YamlStream();
            yaml.Load(reader);

            if (yaml.Documents.Count == 0
                || yaml.Documents[0].RootNode is not YamlMappingNode root
                || !TryGetMapping(root, "services", out var servicesNode))
            {
                continue;
            }

            foreach (var (serviceKeyNode, serviceValueNode) in servicesNode.Children)
            {
                if (serviceKeyNode is not YamlScalarNode { Value: { Length: > 0 } serviceName }
                    || serviceValueNode is not YamlMappingNode serviceNode)
                {
                    continue;
                }

                if (!references.TryGetValue(serviceName, out var serviceReferences))
                {
                    serviceReferences = new Dictionary<string, string>(StringComparer.Ordinal);
                    references[serviceName] = serviceReferences;
                }

                if (TryGetMapping(serviceNode, "environment", out var environmentMapping))
                {
                    foreach (var (environmentKeyNode, environmentValueNode) in environmentMapping.Children)
                    {
                        if (environmentKeyNode is not YamlScalarNode { Value: { Length: > 0 } environmentName })
                            continue;

                        var value = (environmentValueNode as YamlScalarNode)?.Value;
                        SetEnvironmentBindingReference(serviceReferences, environmentName, value);
                    }
                }
                else if (TryGetSequence(serviceNode, "environment", out var environmentSequence))
                {
                    foreach (var item in environmentSequence.Children.OfType<YamlScalarNode>())
                    {
                        var entry = item.Value;
                        if (string.IsNullOrWhiteSpace(entry))
                            continue;

                        var separator = entry.IndexOf('=');
                        var environmentName = (separator < 0 ? entry : entry[..separator]).Trim();
                        if (environmentName.Length == 0)
                            continue;

                        SetEnvironmentBindingReference(
                            serviceReferences,
                            environmentName,
                            separator < 0 ? null : entry[(separator + 1)..]);
                    }
                }
            }
        }

        return references
            .SelectMany(service => service.Value.Select(environment =>
                new StackComposeEnvironmentBindingReference(service.Key, environment.Key, environment.Value)))
            .OrderBy(reference => reference.ServiceName, StringComparer.OrdinalIgnoreCase)
            .ThenBy(reference => reference.EnvironmentName, StringComparer.Ordinal)
            .ToArray();
    }

    private static void SetEnvironmentBindingReference(
        IDictionary<string, string> references,
        string environmentName,
        string? value)
    {
        if (TryGetExactBindingName(value, out var bindingName))
            references[environmentName] = bindingName;
        else
            references.Remove(environmentName);
    }

    private static bool TryGetExactBindingName(string? value, out string bindingName)
    {
        bindingName = string.Empty;
        if (string.IsNullOrWhiteSpace(value))
            return false;

        var expression = value.Trim();
        if (expression.Length >= 4 && expression.StartsWith("${", StringComparison.Ordinal) && expression[^1] == '}')
            bindingName = expression[2..^1];
        else if (expression.Length >= 2 && expression[0] == '$')
            bindingName = expression[1..];
        else
            return false;

        return bindingName.Length is > 0 and <= 128
               && (char.IsAsciiLetter(bindingName[0]) || bindingName[0] == '_')
               && bindingName.Skip(1).All(character => char.IsAsciiLetterOrDigit(character) || character == '_');
    }

    public static StackComposeVolumeResolution ParseVolumes(string composeFile)
        => ParseVolumesFromYaml(composeFile);

    public static StackComposeVolumeResolution ParseVolumeReferences(IReadOnlyCollection<string>? volumeReferences)
    {
        if (volumeReferences is null || volumeReferences.Count == 0)
            return new StackComposeVolumeResolution([], [], false);

        var serviceReferences = new HashSet<string>(StringComparer.Ordinal);
        var hasAnonymousVolumes = false;

        foreach (var volumeReference in volumeReferences)
        {
            var reference = ParseShortVolumeReference(volumeReference);
            if (reference is null)
                continue;

            if (reference.IsAnonymous)
            {
                hasAnonymousVolumes = true;
                continue;
            }

            if (!string.IsNullOrWhiteSpace(reference.Source))
                serviceReferences.Add(reference.Source);
        }

        return new StackComposeVolumeResolution([], [.. serviceReferences], hasAnonymousVolumes);
    }

    private static IReadOnlyDictionary<string, StackComposeService> ParseServicesFromYaml(string composeFile)
    {
        var services = new Dictionary<string, StackComposeService>(StringComparer.OrdinalIgnoreCase);

        if (string.IsNullOrWhiteSpace(composeFile))
            return services;

        using var reader = new StringReader(composeFile);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode root)
            return services;

        if (!TryGetMapping(root, "services", out var servicesNode))
            return services;

        foreach (var (key, value) in servicesNode.Children)
        {
            if (key is not YamlScalarNode serviceKey || string.IsNullOrWhiteSpace(serviceKey.Value))
                continue;

            var image = value is YamlMappingNode serviceNode
                ? TryGetScalar(serviceNode, "image")
                : null;

            var expectedConfigHash = value is YamlMappingNode node
                ? TryGetServiceLabel(node, CitadelLabels.ServiceHash)
                : null;

            services[serviceKey.Value] = new StackComposeService(
                ServiceName: serviceKey.Value,
                Image: image,
                ExpectedConfigHash: expectedConfigHash);
        }

        return services;
    }

    private static StackComposeVolumeResolution ParseVolumesFromYaml(string composeFile)
    {
        var declared = new Dictionary<string, StackComposeDeclaredVolume>(StringComparer.Ordinal);
        var serviceReferences = new HashSet<string>(StringComparer.Ordinal);
        var hasAnonymousVolumes = false;

        if (string.IsNullOrWhiteSpace(composeFile))
            return new StackComposeVolumeResolution([], [], false);

        using var reader = new StringReader(composeFile);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode root)
            return new StackComposeVolumeResolution([], [], false);

        if (TryGetMapping(root, "volumes", out var volumesNode))
        {
            foreach (var (key, value) in volumesNode.Children)
            {
                if (key is not YamlScalarNode volumeKey || string.IsNullOrWhiteSpace(volumeKey.Value))
                    continue;

                var external = value is YamlMappingNode volumeDefinition
                               && IsExternalVolumeDefinition(volumeDefinition);
                declared[volumeKey.Value] = new StackComposeDeclaredVolume(volumeKey.Value, external);
            }
        }

        if (TryGetMapping(root, "services", out var servicesNode))
        {
            foreach (var service in servicesNode.Children.Values.OfType<YamlMappingNode>())
            {
                if (!TryGetSequence(service, "volumes", out var serviceVolumes))
                    continue;

                foreach (var item in serviceVolumes.Children)
                {
                    var reference = ParseServiceVolumeReference(item);
                    if (reference is null)
                        continue;

                    if (reference.IsAnonymous)
                    {
                        hasAnonymousVolumes = true;
                        continue;
                    }

                    if (!string.IsNullOrWhiteSpace(reference.Source))
                        serviceReferences.Add(reference.Source);
                }
            }
        }

        return new StackComposeVolumeResolution(
            [.. declared.Values],
            [.. serviceReferences],
            hasAnonymousVolumes);
    }

    private static bool TryGetMapping(YamlMappingNode node, string key, out YamlMappingNode value)
    {
        foreach (var (childKey, childValue) in node.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase)
                && childValue is YamlMappingNode mapping)
            {
                value = mapping;
                return true;
            }
        }

        value = null!;
        return false;
    }

    private static bool TryGetSequence(YamlMappingNode node, string key, out YamlSequenceNode value)
    {
        foreach (var (childKey, childValue) in node.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase)
                && childValue is YamlSequenceNode sequence)
            {
                value = sequence;
                return true;
            }
        }

        value = null!;
        return false;
    }

    private static string? TryGetScalar(YamlMappingNode node, string key)
    {
        foreach (var (childKey, childValue) in node.Children)
        {
            if (childKey is YamlScalarNode scalar
                && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase)
                && childValue is YamlScalarNode value)
            {
                return value.Value;
            }
        }

        return null;
    }

    private static StackComposeVolumeReference? ParseServiceVolumeReference(YamlNode node)
        => node switch
        {
            YamlScalarNode scalar => ParseShortVolumeReference(scalar.Value),
            YamlMappingNode mapping => ParseLongVolumeReference(mapping),
            _ => null
        };

    private static StackComposeVolumeReference? ParseShortVolumeReference(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
            return null;

        var firstSegment = value.Split(':', 2)[0].Trim();
        if (string.IsNullOrWhiteSpace(firstSegment))
            return null;

        if (!value.Contains(':', StringComparison.Ordinal))
            return new StackComposeVolumeReference(null, IsAnonymous: true);

        if (IsLikelyHostPath(firstSegment))
            return null;

        return new StackComposeVolumeReference(firstSegment, IsAnonymous: false);
    }

    private static StackComposeVolumeReference? ParseLongVolumeReference(YamlMappingNode mapping)
    {
        var type = TryGetScalar(mapping, "type");
        if (!string.IsNullOrWhiteSpace(type)
            && !string.Equals(type, "volume", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        var source = TryGetScalar(mapping, "source") ?? TryGetScalar(mapping, "src");
        if (string.IsNullOrWhiteSpace(source))
            return new StackComposeVolumeReference(null, IsAnonymous: true);

        if (IsLikelyHostPath(source))
            return null;

        return new StackComposeVolumeReference(source.Trim(), IsAnonymous: false);
    }

    private static bool IsExternalVolumeDefinition(YamlMappingNode mapping)
    {
        foreach (var (key, value) in mapping.Children)
        {
            if (key is not YamlScalarNode scalar || !string.Equals(scalar.Value, "external", StringComparison.OrdinalIgnoreCase))
                continue;

            return value switch
            {
                YamlScalarNode external => bool.TryParse(external.Value, out var parsed) && parsed,
                YamlMappingNode => true,
                _ => false
            };
        }

        return false;
    }

    private static bool IsLikelyHostPath(string value)
        => value.StartsWith("/", StringComparison.Ordinal)
           || value.StartsWith("./", StringComparison.Ordinal)
           || value.StartsWith("../", StringComparison.Ordinal)
           || value.StartsWith("~/", StringComparison.Ordinal)
           || value.Contains('\\');

    private static string? TryGetServiceLabel(YamlMappingNode serviceNode, string labelName)
    {
        foreach (var (key, value) in serviceNode.Children)
        {
            if (key is not YamlScalarNode scalar || !string.Equals(scalar.Value, "labels", StringComparison.OrdinalIgnoreCase))
                continue;

            return value switch
            {
                YamlMappingNode mapping => TryGetScalar(mapping, labelName),
                YamlSequenceNode sequence => TryGetSequenceLabel(sequence, labelName),
                _ => null
            };
        }

        return null;
    }

    private static string? TryGetSequenceLabel(YamlSequenceNode sequence, string labelName)
    {
        foreach (var child in sequence.Children.OfType<YamlScalarNode>())
        {
            var value = child.Value;
            if (value is null || !value.StartsWith(labelName, StringComparison.OrdinalIgnoreCase))
                continue;

            if (value.Length == labelName.Length)
                return string.Empty;

            if (value[labelName.Length] == '=')
                return value[(labelName.Length + 1)..];
        }

        return null;
    }
}

internal sealed record StackComposeService(
    string ServiceName,
    string? Image,
    string? ExpectedConfigHash);

internal sealed record StackComposeEnvironmentBindingReference(
    string ServiceName,
    string EnvironmentName,
    string BindingName);

internal sealed record StackComposeVolumeResolution(
    IReadOnlyList<StackComposeDeclaredVolume> DeclaredVolumes,
    IReadOnlyList<string> ServiceVolumeReferences,
    bool HasAnonymousVolumes);

internal sealed record StackComposeDeclaredVolume(string Name, bool IsExternal);

internal sealed record StackComposeVolumeReference(string? Source, bool IsAnonymous);
