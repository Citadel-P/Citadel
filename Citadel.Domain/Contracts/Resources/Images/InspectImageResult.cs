namespace Domain.Contracts.Resources.Images;

public record InspectImageResult(
    string Id,
    string Author,
    string Parent,
    string Comment,
    string Created,
    string DockerVersion,
    string Architecture,
    string OSVersion,
    long VirtualSize,
    string Variant,
    string OS,
    long Size,
    RootFs RootFS,
    Metadata Metadata,
    ImagaConfig Config,
    ImageDescriptor Descriptor,
    ImageGraphicDriver GraphDriver,
    IReadOnlyList<Manifest> Manifests,
    IReadOnlyList<string> RepoTags,
    IReadOnlyList<string> RepoDigests
);


public record ImageDescriptor(
    long Size,
    string? Data,
    string Digest,
    string MediaType,
    string ArtifactType,
    PlatformDescriptor? Platform,
    IReadOnlyList<string> Urls,
    IReadOnlyDictionary<string, string> Annotations
);

public record SizeInfo(long? Total, long? Content, long? Unpacked);

public record PlatformDescriptor(
    string OS,
    string Variant,
    string OSVersion,
    string Architecture,
    IReadOnlyList<string> OSFeatures
);

public record ImageData(PlatformDescriptor? Platform, IReadOnlyList<string> Containers, SizeInfo? Size);

public record AttestationData(string? For);

public record Manifest(
    string Id,
    string Kind,
    bool Available,
    SizeInfo? Size,
    ImageData? ImageData,
    ImageDescriptor Descriptor,
    AttestationData? AttestationData
);

public record ImagaConfig(
    bool Tty,
    string User,
    string Image,
    string Hostname,
    string Domainname,
    bool AttachStdin,
    bool AttachStdout,
    bool AttachStderr,
    string StopSignal,
    bool OpenStdin,
    bool StdinOnce,
    bool ArgsEscaped,
    string WorkingDir,
    ImageHealthCheck? HealthCheck,
    IReadOnlyList<string> Shell,
    IReadOnlyList<string> Env,
    IReadOnlyList<string> Cmd,
    IReadOnlyList<string> OnBuild,
    IReadOnlyList<string> EntryPoint,
    IReadOnlyDictionary<string, Empty> Volumes,
    IReadOnlyDictionary<string, string> Labels,
    IReadOnlyDictionary<string, Empty> ExposedPorts
);

public record ImageHealthCheck(
    IReadOnlyList<string> Test,
    long? Interval,
    long? Timeout,
    long? Retries,
    long? StartPeriod,
    long? StartInterval
);

public record GraphDriverData(
    string MergedDir,
    string UpperDir,
    string WorkDir
);

public record ImageGraphicDriver(string Name, GraphDriverData Data);

public record RootFs(string Type, IReadOnlyList<string> Layers);

public record Metadata(string LastTagTime);
public record Empty();
