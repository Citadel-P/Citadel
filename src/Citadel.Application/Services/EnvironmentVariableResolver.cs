using LightResults;
using System.Text.RegularExpressions;

namespace Application.Services;

internal static partial class EnvironmentVariableResolver
{
    public static IEnumerable<string> GetReferencedNames(IEnumerable<string> configured)
    {
        foreach (var rawLine in configured)
        {
            var line = rawLine.Trim();
            if (string.IsNullOrWhiteSpace(line) || line.StartsWith('#'))
                continue;

            var separator = line.IndexOf('=');
            if (separator < 0)
            {
                yield return line;
                continue;
            }

            foreach (Match match in EnvironmentReferenceRegex().Matches(line[(separator + 1)..]))
                yield return match.Groups["name"].Value;
        }
    }

    public static Result<IReadOnlyList<string>> Build(
        IReadOnlyList<string> configured,
        ResolvedResourceBindings configuration,
        string resourceLabel)
    {
        if (configured.Count == 0)
            return Array.Empty<string>();

        var values = configuration.ToValueDictionary();
        var result = new List<string>(configured.Count);

        foreach (var rawLine in configured)
        {
            var line = rawLine.Trim();
            if (string.IsNullOrWhiteSpace(line) || line.StartsWith('#'))
                continue;

            var separator = line.IndexOf('=');
            if (separator < 0)
            {
                if (!EnvironmentNameRegex().IsMatch(line))
                    return Result.Failure<IReadOnlyList<string>>($"{resourceLabel} environment key '{line}' is not valid.");
                if (!values.TryGetValue(line, out var value))
                    return Result.Failure<IReadOnlyList<string>>($"{resourceLabel} environment key '{line}' is not defined in Variables.");

                result.Add($"{line}={value}");
                continue;
            }

            var name = line[..separator].Trim();
            if (!EnvironmentNameRegex().IsMatch(name))
                return Result.Failure<IReadOnlyList<string>>($"{resourceLabel} environment key '{name}' is not valid.");

            var missingName = string.Empty;
            var interpolated = EnvironmentReferenceRegex().Replace(line[(separator + 1)..], match =>
            {
                var reference = match.Groups["name"].Value;
                if (values.TryGetValue(reference, out var value))
                    return value;

                missingName = reference;
                return match.Value;
            });
            if (!string.IsNullOrEmpty(missingName))
                return Result.Failure<IReadOnlyList<string>>(
                    $"{resourceLabel} environment reference '{missingName}' is not defined in Variables.");

            result.Add($"{name}={interpolated}");
        }

        return result;
    }

    [GeneratedRegex("^[A-Za-z_][A-Za-z0-9_]*$", RegexOptions.Compiled)]
    private static partial Regex EnvironmentNameRegex();

    [GeneratedRegex(@"\$\{(?<name>[A-Za-z_][A-Za-z0-9_]*)\}", RegexOptions.Compiled)]
    private static partial Regex EnvironmentReferenceRegex();
}
