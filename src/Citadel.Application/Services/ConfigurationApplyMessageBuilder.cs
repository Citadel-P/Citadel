using Domain.Entities.Configuration;

namespace Application.Services;

internal static class ConfigurationApplyMessageBuilder
{
    public static string BuildComposeInterpolationMessage(
        ResolvedConfiguration configuration,
        IReadOnlySet<string> referencedKeys,
        int sourceEnvironmentFileCount)
    {
        var usedEntries = configuration.Entries
            .Where(entry => referencedKeys.Contains(entry.Name))
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .ToArray();

        var message = usedEntries.Length == 0
            ? "No Citadel variables or secrets were referenced by the compose files."
            : $"Resolved {FormatEntryGroups(usedEntries)} for compose interpolation.";

        if (sourceEnvironmentFileCount <= 0)
            return message;

        return $"{message} Included {FormatCount(sourceEnvironmentFileCount, "repo env file")}.";
    }

    public static string BuildDeploymentEnvironmentMessage(ResolvedConfiguration configuration)
    {
        var entries = configuration.Entries
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .ToArray();

        return entries.Length == 0
            ? "No Citadel variables or secrets are configured for this deployment."
            : $"Injected {FormatEntryGroups(entries)} into the deployment environment.";
    }

    private static string FormatEntryGroups(IReadOnlyCollection<ResolvedConfigurationEntry> entries)
    {
        var variables = entries
            .Where(entry => entry.Kind == ConfigurationEntryKind.Variable)
            .Select(entry => $"{entry.Name}={FormatValue(entry.Value)}")
            .ToArray();

        var secrets = entries
            .Where(entry => entry.Kind == ConfigurationEntryKind.Secret)
            .Select(FormatSecretName)
            .ToArray();

        return JoinNonEmpty(
            FormatNamedEntries(variables, "variable"),
            FormatNamedEntries(secrets, "secret"));
    }

    private static string FormatNamedEntries(IReadOnlyList<string> names, string singular)
    {
        if (names.Count == 0)
            return string.Empty;

        return $"{FormatCount(names.Count, singular)} {string.Join(", ", names)}";
    }

    private static string JoinNonEmpty(params string[] parts)
    {
        var nonEmpty = parts.Where(part => !string.IsNullOrWhiteSpace(part)).ToArray();
        return nonEmpty.Length switch
        {
            0 => "no Citadel variables or secrets",
            1 => nonEmpty[0],
            _ => string.Join(", ", nonEmpty[..^1]) + " and " + nonEmpty[^1]
        };
    }

    private static string FormatCount(int count, string singular)
        => count == 1 ? $"1 {singular}" : $"{count} {singular}s";

    private static string FormatSecretName(ResolvedConfigurationEntry entry)
    {
        if (string.IsNullOrWhiteSpace(entry.SecretName) || string.Equals(entry.SecretName, entry.Name, StringComparison.Ordinal))
            return entry.Name;

        return $"{entry.SecretName} as {entry.Name}";
    }

    private static string FormatValue(string value)
    {
        var normalized = value
            .Replace("\r", "\\r", StringComparison.Ordinal)
            .Replace("\n", "\\n", StringComparison.Ordinal);

        return normalized.Length <= 80
            ? normalized
            : normalized[..77] + "...";
    }
}
