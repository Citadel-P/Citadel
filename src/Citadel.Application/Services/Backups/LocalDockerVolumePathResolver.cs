namespace Application.Services.Backups;

internal static class LocalDockerVolumePathResolver
{
    private const string HostRootEnvironmentVariable = "CITADEL_HOST_ROOT";
    private const string DefaultHostRoot = "/host";

    public static bool TryResolve(string? dockerMountpoint, out string resolvedPath)
        => TryResolve(
            dockerMountpoint,
            Environment.GetEnvironmentVariable(HostRootEnvironmentVariable) ?? DefaultHostRoot,
            out resolvedPath);

    internal static bool TryResolve(
        string? dockerMountpoint,
        string hostRoot,
        out string resolvedPath)
    {
        resolvedPath = string.Empty;
        if (string.IsNullOrWhiteSpace(dockerMountpoint)
            || dockerMountpoint.Contains('\0'))
        {
            return false;
        }

        try
        {
            var directPath = Path.GetFullPath(dockerMountpoint);
            if (Directory.Exists(directPath))
            {
                resolvedPath = directPath;
                return true;
            }

            if (string.IsNullOrWhiteSpace(hostRoot)
                || hostRoot.Contains('\0')
                || !Path.IsPathFullyQualified(hostRoot)
                || dockerMountpoint[0] != '/'
                || dockerMountpoint.Contains('\\'))
            {
                return false;
            }

            var segments = dockerMountpoint.Split(
                '/',
                StringSplitOptions.RemoveEmptyEntries);
            if (segments.Any(static segment => segment is "." or ".."))
                return false;

            var normalizedHostRoot = Path.GetFullPath(hostRoot);
            var relativeMountpoint = segments.Length == 0
                ? string.Empty
                : Path.Combine(segments);
            var candidate = Path.GetFullPath(
                Path.Combine(normalizedHostRoot, relativeMountpoint));
            var relativeCandidate = Path.GetRelativePath(
                normalizedHostRoot,
                candidate);
            if (Path.IsPathRooted(relativeCandidate)
                || relativeCandidate.Equals("..", StringComparison.Ordinal)
                || relativeCandidate.StartsWith(
                    $"..{Path.DirectorySeparatorChar}",
                    StringComparison.Ordinal)
                || !Directory.Exists(candidate))
            {
                return false;
            }

            resolvedPath = candidate;
            return true;
        }
        catch (Exception ex) when (
            ex is ArgumentException
                or IOException
                or NotSupportedException)
        {
            return false;
        }
    }
}
