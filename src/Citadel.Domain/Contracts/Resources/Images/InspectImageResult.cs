using Domain.Entities;
using Domain.Entities.Registries;

namespace Domain.Contracts.Resources.Images;

public record InspectImageResult(
    string Id,
    long Size,
    string Os,
    string Created,
    string Architecture,
    IEnumerable<string> Env,
    IEnumerable<string> Cmd,
    IEnumerable<string> RepoTags,
    IEnumerable<string> Volumes,
    IEnumerable<string> ExposedPorts,
    IEnumerable<HistoryImageResult> Layers,
    IDictionary<string, string> Labels,
    IEnumerable<ContainerImageResult> Containers,
    string? User = null,
    string? WorkingDir = null,
    IReadOnlyList<string>? EntryPoint = null,
    string? StopSignal = null
    )
{
    public Guid PlatformId { get; set; }
    public string? DockerNodeId { get; set; }
    public Registry? Registry { get; set; }
};

public record ContainerImageResult(
    string Id, 
    string Name,
    ContainerStateStatus State, 
    IEnumerable<string> Volumes,
    Dictionary<string, string> Networks,
    Dictionary<string, IReadOnlyList<HostPortBinding>> Ports
    );

public record InspectImageResult2(
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
    ImageRootFs? RootFS,
    ImageMetadata? Metadata,
    ImageConfig? Config,
    ImageDescriptor? Descriptor,
    ImageGraphicDriver? GraphDriver,
    IReadOnlyList<ImageManifest> Manifests,
    IReadOnlyList<string> RepoTags,
    IReadOnlyList<string> RepoDigests
);

public record ImageDescriptor(
    long Size,
    string? Data,
    string Digest,
    string MediaType,
    string ArtifactType,
    ImagePlatformDescriptor? Platform,
    IReadOnlyList<string> Urls,
    IReadOnlyDictionary<string, string> Annotations
);

public record SizeInfo(long? Total, long? Content, long? Unpacked);

public record ImagePlatformDescriptor(
    string OS,
    string Variant,
    string OSVersion,
    string Architecture,
    IReadOnlyList<string> OSFeatures
);

public record ImageData(ImagePlatformDescriptor? Platform, IReadOnlyList<string> Containers, SizeInfo? Size);

public record AttestationData(string? For);

public record ImageManifest(
    string Id,
    string Kind,
    bool Available,
    SizeInfo? Size,
    ImageData? ImageData,
    ImageDescriptor? Descriptor,
    AttestationData? AttestationData
);

public record ImageConfig(
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

public record ImageGraphDriverData(
    string MergedDir,
    string UpperDir,
    string WorkDir
);

public record ImageGraphicDriver(string Name, ImageGraphDriverData? Data);

public record ImageRootFs(string Type, IReadOnlyList<string> Layers);

public record ImageMetadata(string LastTagTime);
public record Empty();
