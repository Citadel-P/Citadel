namespace Domain.Contracts.Resources.Images;

public sealed record BuildImageCommand(
    string PlatformAddress,
    string ContextDirectory,
    string DockerfilePath,
    IReadOnlyList<string> Tags,
    IReadOnlyDictionary<string, string> BuildArgs,
    string? Target,
    string? RegistryAuth,
    string? RegistryHost,
    TimeSpan Timeout,
    int MaxLineBytes = 16_384,
    byte[]? ContextArchive = null,
    string? DockerfileArchivePath = null,
    IReadOnlyList<BuildImageSecret>? Secrets = null);

public sealed record BuildImageSecret(string Id, string Value);

public sealed record PushImageCommand(
    string PlatformAddress,
    string ImageReference,
    string? RegistryAuth);

public sealed record BuildHostCapabilitiesResult(
    bool Available,
    string? DockerVersion,
    string? ApiVersion,
    string? OperatingSystem,
    string? Architecture,
    string? BuildKitVersion);

public sealed record ImageBuildStreamItem(
    string? Id,
    string? Stream,
    string? Status,
    string? ErrorMessage,
    string? ProgressMessage,
    ImageBuildProgress? Progress,
    ImageBuildError? Error);

public sealed record ImageBuildProgress(
    string? Units,
    long? Current,
    long? Total,
    long? Start);

public sealed record ImageBuildError(long? Code, string? Message);
