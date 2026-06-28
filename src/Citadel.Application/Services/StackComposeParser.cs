using Domain.Entities.Stacks;
using YamlDotNet.RepresentationModel;

namespace Application.Services;

internal static class StackComposeParser
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
