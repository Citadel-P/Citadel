namespace Application.Services;

internal static class ApplicationVersion
{
    public static string CoreInformationalVersion { get; } =
        typeof(ApplicationVersion).Assembly
            .GetCustomAttributes(
                typeof(System.Reflection.AssemblyInformationalVersionAttribute),
                inherit: false)
            .OfType<System.Reflection.AssemblyInformationalVersionAttribute>()
            .SingleOrDefault()?
            .InformationalVersion
        ?? "0.0.0";

    public static string CoreVersion { get; } = GetCoreVersion(CoreInformationalVersion);

    internal static string GetCoreVersion(string? informationalVersion)
    {
        if (string.IsNullOrWhiteSpace(informationalVersion))
            return "0.0.0";

        var numericVersion = informationalVersion.AsSpan();
        var suffixIndex = numericVersion.IndexOfAny('-', '+');
        if (suffixIndex >= 0)
            numericVersion = numericVersion[..suffixIndex];

        if (!Version.TryParse(numericVersion, out var version))
            return "0.0.0";

        return $"{version.Major}.{version.Minor}.{Math.Max(version.Build, 0)}";
    }
}
