using Domain.Contracts.Resources.Containers;
using System.Security.Cryptography;
using System.Text;
using YamlDotNet.RepresentationModel;

namespace Application.Services;

internal static class ComposeLabels
{
    public const string Project = "com.docker.compose.project";
    public const string Service = "com.docker.compose.service";
}

internal static class CitadelLabels
{
    public const string Prefix = "x-citadel.";
    public const string Managed = Prefix + "managed";
    public const string StackId = Prefix + "stack-id";
    public const string ReleaseId = Prefix + "release-id";
    public const string ServiceHash = Prefix + "service-hash";
}

internal static class StackContainerOwnership
{
    public static ContainerFilterCommand CreateOwnedContainerFilter(
        string platformAddress,
        string projectName,
        Guid stackId,
        bool all = true)
        => new(
            PlatformAddress: platformAddress,
            All: all,
            Filters: new Dictionary<string, IDictionary<string, bool>>
            {
                ["label"] = new Dictionary<string, bool>
                {
                    [$"{ComposeLabels.Project}={projectName}"] = true,
                    [$"{CitadelLabels.Managed}=true"] = true,
                    [$"{CitadelLabels.StackId}={FormatStackId(stackId)}"] = true
                }
            });

    public static bool IsOwnedByStack(IReadOnlyDictionary<string, string> labels, Guid stackId)
        => labels.TryGetValue(CitadelLabels.Managed, out var managed)
           && string.Equals(managed, "true", StringComparison.OrdinalIgnoreCase)
           && labels.TryGetValue(CitadelLabels.StackId, out var owner)
           && string.Equals(owner, FormatStackId(stackId), StringComparison.OrdinalIgnoreCase);

    public static bool IsCitadelManaged(IReadOnlyDictionary<string, string> labels)
        => labels.TryGetValue(CitadelLabels.Managed, out var managed)
           && string.Equals(managed, "true", StringComparison.OrdinalIgnoreCase);

    public static string FormatStackId(Guid stackId) => stackId.ToString("D");
}

internal static class StackComposeLabelInjector
{
    public static string Inject(string composeFile, Guid stackId, Guid releaseId)
    {
        using var reader = new StringReader(composeFile);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode root)
        {
            return composeFile;
        }

        if (!TryGetMapping(root, "services", out var services))
        {
            return composeFile;
        }

        foreach (var (_, value) in services.Children)
        {
            if (value is not YamlMappingNode service)
            {
                continue;
            }

            var labels = GetOrCreateNormalizedLabels(service);
            var serviceHash = HashService(service);

            SetMappingValue(labels, CitadelLabels.Managed, "true");
            SetMappingValue(labels, CitadelLabels.StackId, StackContainerOwnership.FormatStackId(stackId));
            SetMappingValue(labels, CitadelLabels.ReleaseId, releaseId.ToString("D"));
            SetMappingValue(labels, CitadelLabels.ServiceHash, serviceHash);
        }

        using var writer = new StringWriter();
        yaml.Save(writer, assignAnchors: false);
        return writer.ToString();
    }

    private static YamlMappingNode GetOrCreateNormalizedLabels(YamlMappingNode service)
    {
        var labelKey = FindKey(service, "labels");
        if (labelKey is null)
        {
            var newLabels = new YamlMappingNode();
            service.Add("labels", newLabels);
            return newLabels;
        }

        var labelsNode = service.Children[labelKey];
        var labels = labelsNode switch
        {
            YamlMappingNode mapping => ValidateLabelMapping(mapping),
            YamlSequenceNode sequence => ConvertLabelSequence(sequence),
            _ => throw new InvalidOperationException("Compose service labels must be a mapping or a sequence.")
        };

        service.Children[labelKey] = labels;
        return labels;
    }

    private static YamlMappingNode ValidateLabelMapping(YamlMappingNode mapping)
    {
        foreach (var key in mapping.Children.Keys.OfType<YamlScalarNode>())
        {
            RejectReservedLabel(key.Value);
        }

        return mapping;
    }

    private static YamlMappingNode ConvertLabelSequence(YamlSequenceNode sequence)
    {
        var mapping = new YamlMappingNode();
        foreach (var child in sequence.Children)
        {
            if (child is not YamlScalarNode scalar || scalar.Value is null)
            {
                throw new InvalidOperationException("Compose service label sequence items must be strings.");
            }

            var separatorIndex = scalar.Value.IndexOf('=', StringComparison.Ordinal);
            var key = separatorIndex >= 0 ? scalar.Value[..separatorIndex] : scalar.Value;
            var value = separatorIndex >= 0 ? scalar.Value[(separatorIndex + 1)..] : string.Empty;

            RejectReservedLabel(key);
            SetMappingValue(mapping, key, value);
        }

        return mapping;
    }

    private static void RejectReservedLabel(string? key)
    {
        if (!string.IsNullOrWhiteSpace(key)
            && key.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException($"Compose service label '{key}' is reserved by Citadel.");
        }
    }

    private static string HashService(YamlMappingNode service)
    {
        var serialized = SerializeNode(service);
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(serialized));
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    private static string SerializeNode(YamlNode node)
    {
        using var writer = new StringWriter();
        var stream = new YamlStream(new YamlDocument(node));
        stream.Save(writer, assignAnchors: false);
        return writer.ToString();
    }

    private static bool TryGetMapping(YamlMappingNode node, string key, out YamlMappingNode value)
    {
        var yamlKey = FindKey(node, key);
        if (yamlKey is not null && node.Children[yamlKey] is YamlMappingNode mapping)
        {
            value = mapping;
            return true;
        }

        value = null!;
        return false;
    }

    private static YamlNode? FindKey(YamlMappingNode node, string key)
        => node.Children.Keys.FirstOrDefault(childKey =>
            childKey is YamlScalarNode scalar
            && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase));

    private static void SetMappingValue(YamlMappingNode node, string key, string value)
    {
        var existing = FindKey(node, key);
        if (existing is not null)
        {
            node.Children[existing] = new YamlScalarNode(value);
            return;
        }

        node.Add(key, value);
    }
}
