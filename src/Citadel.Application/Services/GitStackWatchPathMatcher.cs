using Domain.Entities.Stacks;

namespace Application.Services;

internal static class GitStackWatchPathMatcher
{
    public static bool HasRelevantChanges(
        GitStack spec,
        StackReleaseSource source,
        IReadOnlyCollection<string> changedPaths)
    {
        if (changedPaths.Count == 0)
        {
            return false;
        }

        var watchPaths = BuildWatchPaths(spec, source);
        if (watchPaths.Count == 0)
        {
            return true;
        }

        foreach (var changedPath in changedPaths)
        {
            var normalizedChangedPath = NormalizePath(changedPath);
            if (string.IsNullOrWhiteSpace(normalizedChangedPath))
                continue;

            if (watchPaths.Any(watchPath => Matches(watchPath, normalizedChangedPath)))
                return true;
        }

        return false;
    }

    public static IReadOnlyList<string> BuildWatchPaths(GitStack spec, StackReleaseSource source)
    {
        var explicitWatchPaths = spec.WatchPaths ?? source.WatchPaths;
        if (explicitWatchPaths is { Count: > 0 })
        {
            return explicitWatchPaths
                .Select(NormalizePath)
                .Where(path => !string.IsNullOrWhiteSpace(path))
                .Select(path => path!)
                .Distinct(StringComparer.Ordinal)
                .ToArray();
        }

        var watchPaths = new List<string>();
        var workingDirectory = NormalizePath(spec.WorkingDirectory ?? source.WorkingDirectory)
            ?? GetParentPath((spec.ComposePaths ?? source.ComposePaths).FirstOrDefault());
        if (!string.IsNullOrWhiteSpace(workingDirectory) && workingDirectory != ".")
        {
            watchPaths.Add($"{workingDirectory}/**");
        }

        AddRange(watchPaths, spec.ComposePaths);
        AddRange(watchPaths, source.ComposePaths);
        AddRange(watchPaths, spec.ComposeEnvFilesFromRepo ?? spec.AdditionalEnvFileFromRepo);
        AddRange(watchPaths, source.ComposeEnvFilesFromRepo ?? source.EnvFilePaths);

        return watchPaths
            .Select(NormalizePath)
            .Where(path => !string.IsNullOrWhiteSpace(path))
            .Select(path => path!)
            .Distinct(StringComparer.Ordinal)
            .ToArray();
    }

    private static void AddRange(List<string> watchPaths, IReadOnlyCollection<string>? paths)
    {
        if (paths is null)
            return;

        foreach (var path in paths)
        {
            watchPaths.Add(path);
        }
    }

    private static bool Matches(string watchPath, string changedPath)
    {
        if (watchPath == "." || watchPath == "**")
        {
            return true;
        }

        if (watchPath.EndsWith("/**", StringComparison.Ordinal))
        {
            var prefix = watchPath[..^3].TrimEnd('/');
            return changedPath.Equals(prefix, StringComparison.Ordinal)
                || changedPath.StartsWith($"{prefix}/", StringComparison.Ordinal);
        }

        return changedPath.Equals(watchPath, StringComparison.Ordinal)
            || changedPath.StartsWith($"{watchPath.TrimEnd('/')}/", StringComparison.Ordinal);
    }

    private static string? NormalizePath(string? path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return null;

        var normalized = path.Replace('\\', '/').Trim().TrimStart('/');
        while (normalized.Contains("//", StringComparison.Ordinal))
        {
            normalized = normalized.Replace("//", "/", StringComparison.Ordinal);
        }

        return normalized.TrimEnd('/');
    }

    private static string? GetParentPath(string? path)
    {
        var normalized = NormalizePath(path);
        if (string.IsNullOrWhiteSpace(normalized))
            return null;

        var separatorIndex = normalized.LastIndexOf('/');
        return separatorIndex <= 0 ? "." : normalized[..separatorIndex];
    }
}
