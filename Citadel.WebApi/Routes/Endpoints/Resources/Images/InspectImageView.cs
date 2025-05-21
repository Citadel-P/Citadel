using Agent.Server.Images;

namespace WebApi.Routes.Endpoints.Resources.Images;

public record InspectImageView(
    string Id,
    DescriptorView Descriptor,
    List<ManifestView> Manifests,
    List<string> RepoTags,
    List<string> RepoDigests,
    string Parent,
    string Comment,
    string Created,
    string DockerVersion,
    string Author,
    ConfigView? Config,
    string Architecture,
    string Variant,
    string Os,
    string OsVersion,
    long Size,
    long VirtualSize,
    GraphDriverView GraphDriver,
    RootFSView RootFS,
    MetadataView Metadata
)
{
    internal static InspectImageView Map(InspectImageReply image)
    {
        return new InspectImageView(
            Id: image.Id,
            Descriptor: image.Descriptor_.Map(),
            Manifests: image.Manifests?.Select(InspectImageViewExtensions.Map).ToList() ?? [],
            RepoTags: image.RepoTags?.ToList() ?? [],
            RepoDigests: image.RepoDigests?.ToList() ?? [],
            Parent: image.Parent,
            Comment: image.Comment,
            Created: image.Created,
            DockerVersion: image.DockerVersion,
            Author: image.Author,
            Config: image.Config?.Map(),
            Architecture: image.Architecture,
            Variant: image.Variant,
            Os: image.Os,
            OsVersion: image.OsVersion,
            Size: image.Size,
            VirtualSize: image.VirtualSize,
            GraphDriver: image.GraphDriver.Map(),
            RootFS: image.RootFS.Map(),
            Metadata: image.Metadata.Map()
        );
    }   
}

public record DescriptorView(
    string MediaType,
    string Digest,
    long Size,
    List<string> Urls,
    Dictionary<string, string> Annotations,
    string Data,
    PlatformDescriptorView? Platform,
    string ArtifactType
);

public record SizeView(
    long? Total, 
    long? Content, 
    long? Unpacked
);

public record PlatformDescriptorView(
    string Architecture,
    string Os,
    string OsVersion,
    List<string> OsFeatures,
    string Variant
);

public record ImageDataView(
    PlatformDescriptorView? Platform,
    List<string>? Containers,
    SizeView? Size
);

public record AttestationDataView(
    string For
);

public record ManifestView(
    string Id,
    DescriptorView? Descriptor,
    bool Available,
    SizeView? Size,
    string Kind,
    ImageDataView? ImageData,
    AttestationDataView? AttestationData
);

public record ConfigView(
    string Hostname,
    string Domainname,
    string User,
    bool AttachStdin,
    bool AttachStdout,
    bool AttachStderr,
    Dictionary<string, Empty> ExposedPorts,
    bool Tty,
    bool OpenStdin,
    bool StdinOnce,
    List<string> Env,
    List<string> Cmd,
    HealthcheckView? Healthcheck,
    bool ArgsEscaped,
    string Image,
    Dictionary<string, Empty> Volumes,
    string WorkingDir,
    List<string> Entrypoint,
    List<string> OnBuild,
    Dictionary<string, string> Labels,
    string StopSignal,
    List<string> Shell
);

public record HealthcheckView(
    List<string> Test,
    long? Interval,
    long? Timeout,
    long? Retries,
    long? StartPeriod,
    long? StartInterval
);

public record GraphDriverDataView(
    string MergedDir,
    string UpperDir,
    string WorkDir
);

public record GraphDriverView(
    string Name,
    GraphDriverDataView? Data
);

public record RootFSView(
    string Type,
    List<string> Layers
);

public record MetadataView(string LastTagTime);

public record Empty();

internal static class InspectImageViewExtensions
{
    public static DescriptorView Map(this DescriptorMessage descriptor) => 
        new (
                MediaType: descriptor.MediaType,
                Digest: descriptor.Digest,
                Size: descriptor.Size,
                Urls: descriptor.Urls?.ToList() ?? [],
                Annotations: descriptor.Annotations?.ToDictionary(x => x.Key, x => x.Value) ?? [],
                Data: descriptor.Data,
                Platform: descriptor.Platform == null ? null : new PlatformDescriptorView(
                    Architecture: descriptor.Platform.Architecture,
                    Os: descriptor.Platform.Os,
                    OsVersion: descriptor.Platform.OsVersion,
                    OsFeatures: descriptor.Platform.OsFeatures?.ToList() ?? [],
                    Variant: descriptor.Platform.Variant
                ),
                ArtifactType: descriptor.ArtifactType
            );

    public static ManifestView Map(this ManifestMessage manifest) =>
        new (
               Id: manifest.Id,
               Descriptor: manifest.Descriptor_?.Map(),
               Available: manifest.Available,
               Size: manifest.Size == null ? null : new SizeView(
                   Total: manifest.Size.Total,
                   Content: manifest.Size.Content,
                   Unpacked: manifest.Size.Unpacked
               ),
               Kind: manifest.Kind,
               ImageData: manifest.ImageData == null ? null : new ImageDataView(
                   Platform: manifest.ImageData.Platform == null ? null : new PlatformDescriptorView(
                       Architecture: manifest.ImageData.Platform.Architecture,
                       Os: manifest.ImageData.Platform.Os,
                       OsVersion: manifest.ImageData.Platform.OsVersion,
                       OsFeatures: manifest.ImageData.Platform.OsFeatures?.ToList() ?? [],
                       Variant: manifest.ImageData.Platform.Variant
                   ),
                   Containers: manifest.ImageData.Containers?.ToList() ?? [],
                   Size: manifest.ImageData.Size == null ? null : new SizeView(
                       Total: manifest.ImageData.Size.Total,
                       Content: manifest.ImageData.Size.Content,
                       Unpacked: manifest.ImageData.Size.Unpacked
                   )
               ),
               AttestationData: manifest.AttestationData == null ? null : new AttestationDataView(
                   For: manifest.AttestationData.For
               )
           );

    internal static ConfigView Map(this ConfigMessage config) => 
        new (
                Hostname: config.Hostname,
                Domainname: config.Domainname,
                User: config.User,
                AttachStdin: config.AttachStdin,
                AttachStdout: config.AttachStdout,
                AttachStderr: config.AttachStderr,
                ExposedPorts: config.ExposedPorts?.ToDictionary(x => x.Key, x => new Empty()) ?? [],
                Tty: config.Tty,
                OpenStdin: config.OpenStdin,
                StdinOnce: config.StdinOnce,
                Env: config.Env?.ToList() ?? [],
                Cmd: config.Cmd?.ToList() ?? [],
                Healthcheck: config.Healthcheck == null ? null : new HealthcheckView(
                    Test: config.Healthcheck.Test?.ToList() ?? [],
                    Interval: config.Healthcheck.Interval,
                    Timeout: config.Healthcheck.Timeout,
                    Retries: config.Healthcheck.Retries,
                    StartPeriod: config.Healthcheck.StartPeriod,
                    StartInterval: config.Healthcheck.StartInterval
                ),
                ArgsEscaped: config.ArgsEscaped,
                Image: config.Image,
                Volumes: config.Volumes?.ToDictionary(x => x.Key, x => new Empty()) ?? [],
                WorkingDir: config.WorkingDir,
                Entrypoint: config.Entrypoint?.ToList() ?? [],
                OnBuild: config.OnBuild?.ToList() ?? [],
                Labels: config.Labels?.ToDictionary(x => x.Key, x => x.Value) ?? [],
                StopSignal: config.StopSignal,
                Shell: config.Shell?.ToList() ?? []
            );

    internal static GraphDriverView Map(this GraphDriverMessage graphDriver) =>
        new (
                Name: graphDriver.Name,
                Data: graphDriver.Data == null ? null : new GraphDriverDataView(
                    MergedDir: graphDriver.Data.MergedDir,
                    UpperDir: graphDriver.Data.UpperDir,
                    WorkDir: graphDriver.Data.WorkDir
                )
            );

    internal static RootFSView Map(this RootFSMessage rootFS) => 
        new (
                Type: rootFS.Type,
                Layers: rootFS.Layers?.ToList() ?? []
            );

    internal static MetadataView Map(this MetadataMessage metadata) => new (LastTagTime: metadata.LastTagTime);
}