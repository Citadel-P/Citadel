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
    string? DockerfileArchivePath = null);

public sealed record PushImageCommand(
    string PlatformAddress,
    string ImageReference,
    string? RegistryAuth);

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
