using Domain.Entities.Stacks;
using LightResults;
using YamlDotNet.RepresentationModel;

namespace Application.Services.Builds;

internal interface IStackBuildImageBindingResolver
{
    Task<Result<ResolvedStackBuildImageBindings>> ResolveAsync(
        IReadOnlyList<StackBuildImageBinding>? bindings,
        CancellationToken cancellationToken);

    string ApplyToComposeContent(string composeContent, IReadOnlyList<ResolvedStackBuildImageBinding> bindings);

    string CreateComposeOverride(IReadOnlyList<ResolvedStackBuildImageBinding> bindings);
}

internal sealed class StackBuildImageBindingResolver(IBuildImageResolver buildImageResolver) : IStackBuildImageBindingResolver
{
    public async Task<Result<ResolvedStackBuildImageBindings>> ResolveAsync(
        IReadOnlyList<StackBuildImageBinding>? bindings,
        CancellationToken cancellationToken)
    {
        if (bindings is not { Count: > 0 })
            return new ResolvedStackBuildImageBindings([], []);

        var resolved = new List<ResolvedStackBuildImageBinding>();
        var messages = new List<string>();
        foreach (var binding in bindings)
        {
            if (string.IsNullOrWhiteSpace(binding.ServiceName))
                return Result.Failure<ResolvedStackBuildImageBindings>("Stack build image binding service name is required.");

            if (binding.BuildProjectId == Guid.Empty)
                return Result.Failure<ResolvedStackBuildImageBindings>($"Build project is required for service '{binding.ServiceName}'.");

            var image = await buildImageResolver.ResolveLatestAsync(binding.BuildProjectId, cancellationToken);
            if (image.IsFailure(out var error, out var resolvedImage))
                return Result.Failure<ResolvedStackBuildImageBindings>($"Service '{binding.ServiceName}': {error.Message}");

            resolved.Add(new ResolvedStackBuildImageBinding(binding, resolvedImage));
            messages.Add($"Resolved service \"{binding.ServiceName}\" from build \"{resolvedImage.ProjectName}\".");
        }

        return new ResolvedStackBuildImageBindings(resolved, messages);
    }

    public string ApplyToComposeContent(string composeContent, IReadOnlyList<ResolvedStackBuildImageBinding> bindings)
    {
        if (bindings.Count == 0 || string.IsNullOrWhiteSpace(composeContent))
            return composeContent;

        using var reader = new StringReader(composeContent);
        var yaml = new YamlStream();
        yaml.Load(reader);

        if (yaml.Documents.Count == 0 || yaml.Documents[0].RootNode is not YamlMappingNode root)
            return composeContent;

        var servicesNode = root.Children
            .FirstOrDefault(static item => item.Key is YamlScalarNode key && string.Equals(key.Value, "services", StringComparison.OrdinalIgnoreCase))
            .Value as YamlMappingNode;
        if (servicesNode is null)
            return composeContent;

        foreach (var binding in bindings)
        {
            var serviceEntry = servicesNode.Children
                .FirstOrDefault(item => item.Key is YamlScalarNode key && string.Equals(key.Value, binding.Binding.ServiceName, StringComparison.OrdinalIgnoreCase));
            if (serviceEntry.Value is not YamlMappingNode serviceNode)
                continue;

            SetScalar(serviceNode, "image", binding.Image.ImageReference);
        }

        using var writer = new StringWriter();
        yaml.Save(writer, assignAnchors: false);
        return writer.ToString();
    }

    public string CreateComposeOverride(IReadOnlyList<ResolvedStackBuildImageBinding> bindings)
    {
        if (bindings.Count == 0)
            return "services: {}\n";

        var lines = new List<string> { "services:" };
        foreach (var binding in bindings)
        {
            lines.Add($"  {QuoteYaml(binding.Binding.ServiceName)}:");
            lines.Add($"    image: {QuoteYaml(binding.Image.ImageReference)}");
        }

        return string.Join(Environment.NewLine, lines) + Environment.NewLine;
    }

    private static void SetScalar(YamlMappingNode node, string key, string value)
    {
        var existing = node.Children.Keys
            .FirstOrDefault(item => item is YamlScalarNode scalar && string.Equals(scalar.Value, key, StringComparison.OrdinalIgnoreCase));
        var keyNode = existing ?? new YamlScalarNode(key);
        node.Children[keyNode] = new YamlScalarNode(value);
    }

    private static string QuoteYaml(string value)
        => "\"" + value.Replace("\\", "\\\\", StringComparison.Ordinal).Replace("\"", "\\\"", StringComparison.Ordinal) + "\"";
}

internal sealed record ResolvedStackBuildImageBindings(
    IReadOnlyList<ResolvedStackBuildImageBinding> Bindings,
    IReadOnlyList<string> Messages);

internal sealed record ResolvedStackBuildImageBinding(
    StackBuildImageBinding Binding,
    ResolvedBuildImage Image);
