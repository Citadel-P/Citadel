using Domain.Entities.Stacks;

namespace Application.Features.Stacks.Commands;

internal static class GitStackSpecValidation
{
    public static string? Validate(GitStack spec)
    {
        if (spec.GitRepoId == Guid.Empty)
            return "Git repository is required.";

        if (string.IsNullOrWhiteSpace(spec.Branch))
            return "Git stack branch is required.";

        if (spec.ComposePaths is not { Count: > 0 })
            return "At least one compose path is required for Git stacks.";

        foreach (var composePath in spec.ComposePaths)
        {
            var error = ValidateRepositoryPath(composePath, "Compose path", allowWildcardSuffix: false);
            if (error is not null)
                return error;
        }

        if (!string.IsNullOrWhiteSpace(spec.WorkingDirectory))
        {
            var error = ValidateRepositoryPath(spec.WorkingDirectory, "Working directory", allowWildcardSuffix: false);
            if (error is not null)
                return error;
        }

        foreach (var envPath in spec.ComposeEnvFilesFromRepo ?? spec.AdditionalEnvFileFromRepo ?? [])
        {
            var error = ValidateRepositoryPath(envPath, "Environment file path", allowWildcardSuffix: false);
            if (error is not null)
                return error;
        }

        foreach (var watchPath in spec.WatchPaths ?? [])
        {
            var error = ValidateRepositoryPath(watchPath, "Watch path", allowWildcardSuffix: true);
            if (error is not null)
                return error;
        }

        return null;
    }

    private static string? ValidateRepositoryPath(string? path, string fieldName, bool allowWildcardSuffix)
    {
        if (string.IsNullOrWhiteSpace(path))
            return $"{fieldName} cannot be empty.";

        var normalized = path.Replace('\\', '/').Trim();
        var pathToValidate = allowWildcardSuffix && normalized.EndsWith("/**", StringComparison.Ordinal)
            ? normalized[..^3]
            : normalized;

        if (string.IsNullOrWhiteSpace(pathToValidate))
            return $"{fieldName} cannot be empty.";

        if (Path.IsPathRooted(pathToValidate)
            || pathToValidate.StartsWith("/", StringComparison.Ordinal)
            || pathToValidate.StartsWith("\\", StringComparison.Ordinal)
            || IsWindowsRootedPath(pathToValidate))
        {
            return $"{fieldName} '{path}' must be relative to the repository root.";
        }

        var depth = 0;
        foreach (var segment in pathToValidate.Split('/', StringSplitOptions.RemoveEmptyEntries))
        {
            if (segment == ".")
                continue;

            if (segment == "..")
            {
                if (depth == 0)
                    return $"{fieldName} '{path}' escapes the repository root.";

                depth--;
                continue;
            }

            depth++;
        }

        return null;
    }

    private static bool IsWindowsRootedPath(string path)
        => path.Length >= 2
        && char.IsLetter(path[0])
        && path[1] == ':';
}
