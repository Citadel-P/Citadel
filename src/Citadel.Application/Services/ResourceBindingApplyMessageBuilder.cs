using Domain;

namespace Application.Services;

internal static class ResourceBindingApplyMessageBuilder
{
    public static string BuildComposeInterpolationMessage(
        ResolvedResourceBindings configuration,
        IReadOnlySet<string> referencedKeys,
        int sourceEnvironmentFileCount)
    {
        var usedEntries = configuration.Entries
            .Where(entry => referencedKeys.Contains(entry.Name))
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .ToArray();
        var mountedSecrets = configuration.Entries
            .Where(entry => entry.Kind == ResourceBindingKind.Secret
                && entry.SecretDeliveryMode == SecretDeliveryMode.MountedFile)
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .Select(FormatSecretName)
            .ToArray();

        var message = usedEntries.Length == 0
            ? "No Citadel variables or secrets were referenced by the compose files."
            : $"Resolved {FormatEntryGroups(usedEntries)} for compose interpolation.";

        if (mountedSecrets.Length > 0)
        {
            var mountedMessage = $"Mounted {FormatNamedEntries(mountedSecrets, "secret file")}.";
            message = usedEntries.Length == 0
                ? mountedMessage
                : $"{message} {mountedMessage}";
        }

        if (sourceEnvironmentFileCount <= 0)
            return message;

        return $"{message} Included {FormatCount(sourceEnvironmentFileCount, "repo env file")}.";
    }

    public static string BuildDeploymentEnvironmentMessage(ResolvedResourceBindings configuration)
    {
        var entries = configuration.Entries
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .ToArray();

        return entries.Length == 0
            ? "No Citadel variables or secrets are configured for this deployment."
            : $"Injected {FormatEntryGroups(entries)} into the deployment environment.";
    }

    public static string BuildServiceEnvironmentMessage(ResolvedResourceBindings configuration)
    {
        var entries = configuration.Entries
            .OrderBy(entry => entry.Name, StringComparer.Ordinal)
            .ToArray();

        return entries.Length == 0
            ? "No Citadel variables or secrets were referenced by this Service."
            : $"Injected {FormatEntryGroups(entries)} into the Service environment.";
    }

    private static string FormatEntryGroups(IReadOnlyCollection<ResolvedResourceBinding> entries)
    {
        var variables = entries
            .Where(entry => entry.Kind == ResourceBindingKind.Variable)
            .Select(entry => $"{entry.Name}={FormatValue(entry.Value)}")
            .ToArray();

        var secrets = entries
            .Where(entry => entry.Kind == ResourceBindingKind.Secret)
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

    private static string FormatSecretName(ResolvedResourceBinding entry)
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
