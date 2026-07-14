using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

internal sealed class VolumePathNormalizer : IVolumePathNormalizer
{
    private const int MaxPathLength = 4096;

    public Result<NormalizedVolumePath> Normalize(string? path)
    {
        if (string.IsNullOrEmpty(path) || path == "/")
            return new NormalizedVolumePath("/", []);

        if (path.Length > MaxPathLength)
            return Invalid("Volume path is too long.");

        if (path[0] != '/')
            return Invalid("Volume path must start with '/'.");

        if (path.Contains('\\', StringComparison.Ordinal))
            return Invalid("Volume path must use '/' separators.");

        if (path.IndexOf('\0') >= 0)
            return Invalid("Volume path contains an invalid NUL character.");

        for (var i = 0; i < path.Length; i++)
        {
            if (char.IsControl(path[i]))
                return Invalid("Volume path contains a control character.");
        }

        if (path.Contains("//", StringComparison.Ordinal))
            return Invalid("Volume path must not contain repeated separators.");

        var rawSegments = path.Split('/', StringSplitOptions.None);
        var segments = new List<string>(rawSegments.Length - 1);

        for (var i = 1; i < rawSegments.Length; i++)
        {
            var segment = rawSegments[i];
            if (segment.Length == 0)
                return Invalid("Volume path must not end with '/' or contain empty segments.");

            if (segment == ".")
                return Invalid("Volume path must not contain '.' segments.");

            if (segment == "..")
                return Invalid("Volume path must not contain '..' segments.");

            segments.Add(segment);
        }

        return new NormalizedVolumePath("/" + string.Join('/', segments), segments);
    }

    private static Result<NormalizedVolumePath> Invalid(string message)
        => Result.Failure<NormalizedVolumePath>(new BadRequestError(message));
}
