using Citadel.Agent.Images.V1;
using Domain.Contracts.Resources.Images;
using Google.Protobuf.Collections;
using static Citadel.Agent.Images.V1.ConfigMessage.Types;

namespace Infrastructure.Connectors.Mappers;

internal static class ImageMappers
{
    internal static List<ImageResult> Map(this IEnumerable<ImageReply> images)
        => [.. images.Select(Map)];

    internal static ImageResult Map(this ImageReply image)
        => new 
        (
            Id: image.Id,
            Size: image.Size,
            Containers: image.Containers,
            ParentId: image.ParentId,
            SharedSize: image.SharedSize,
            Created: image.Created,
            VirtualSize: image.VirtualSize,
            RepoTags: image.RepoTags?.ToList() ?? [],
            RepoDigests: image.RepoDigests?.ToList() ?? [],
            Labels: image.Labels?.ToDictionary() ?? []
        );

    internal static DeleteImageResult Map(this DeleteImageResponse response) 
        => new
        (
            Items: [.. response.Items.Select(item => new Domain.Contracts.Resources.Images.DeleteImageResponseItem(item.Result.ToDictionary()))]
        );

    internal static InspectImageResult Map(this InspectImageResponse image)
        => new(
            Id: image.Id,
            Size: image.Size,
            Os: image.Os,
            Architecture: image.Architecture,
            Created: image.Created,
            Env: image.Env?.ToList() ?? [],
            Cmd: image.Cmd?.ToList() ?? [],
            RepoTags: image.RepoTags?.ToList() ?? [],
            Containers: image.Containers?.Map() ?? [],
            Volumes: image.Volumes?.ToList() ?? [],
            Labels: image.Labels ?? [],
            ExposedPorts: image.ExposedPorts?.ToList() ?? [],
            Layers: image.Layers?.Select(Map)?.ToList() ?? []
        );

    private static IEnumerable<Domain.Contracts.Resources.Images.ContainerImageResult> Map(this RepeatedField<global::Citadel.Agent.Images.V1.ContainerImageResult> containers)
        => containers.Select(Map);

    private static Domain.Contracts.Resources.Images.ContainerImageResult Map(this global::Citadel.Agent.Images.V1.ContainerImageResult container)
        => new (
            Id: container.Id,
            Name: container.Name,
            State: container.State.Map(),
            Volumes: container.Volumes?.ToList() ?? [],
            Networks: container.Networks?.ToDictionary() ?? [],
            Ports: container.Ports?.Map() ?? []
        );

    internal static ImageRootFs Map(this RootFSMessage rootFs) => new(
        Type: rootFs.Type,
        Layers: rootFs.Layers?.ToList() ?? []
    );

    internal static ImageMetadata Map(this MetadataMessage metadata) 
        => new
        (
            LastTagTime: metadata.LastTagTime
        );

    internal static ImageConfig Map(this ConfigMessage config) 
        => new(
            Tty: config.Tty,
            User: config.User,
            Image: config.Image,
            Hostname: config.Hostname,
            Domainname: config.Domainname,
            AttachStdin: config.AttachStdin,
            AttachStdout: config.AttachStdout,
            AttachStderr: config.AttachStderr,
            StopSignal: config.StopSignal,
            OpenStdin: config.OpenStdin,
            StdinOnce: config.StdinOnce,
            ArgsEscaped: config.ArgsEscaped,
            WorkingDir: config.WorkingDir,
            HealthCheck: config.Healthcheck is not null ? config.Healthcheck.Map() : null,
            Shell: config.Shell?.ToList() ?? [],
            Env: config.Env?.ToList() ?? [],
            Cmd: config.Cmd?.ToList() ?? [],
            OnBuild: config.OnBuild?.ToList() ?? [],
            EntryPoint: config.Entrypoint?.ToList() ?? [],
            Volumes: config.Volumes?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? [],
            Labels: config.Labels?.ToDictionary() ?? [],
            ExposedPorts: config.ExposedPorts?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? []
        );

    internal static ImageDescriptor Map(this DescriptorMessage descriptor)
        => new(
            Size: descriptor.Size,
            Data: descriptor.Data,
            Digest: descriptor.Digest,
            MediaType: descriptor.MediaType,
            ArtifactType: descriptor.ArtifactType,
            Platform: descriptor.Platform is not null ? descriptor.Platform.Map() : null,
            Urls: descriptor.Urls?.ToList() ?? [],
            Annotations: descriptor.Annotations?.ToDictionary() ?? []
        );

    internal static ImageGraphicDriver Map(this GraphDriverMessage graphDriver) 
        => new(
            Name: graphDriver.Name,
            Data: graphDriver.Data?.Map()
        );

    internal static ImageGraphDriverData Map(this GraphDriverDataMessage graphDriverData) => new(
        MergedDir: graphDriverData.MergedDir,
        UpperDir: graphDriverData.UpperDir,
        WorkDir: graphDriverData.WorkDir
    );

    internal static ImageManifest Map(this ManifestMessage manifest) => new(
        Id: manifest.Id,
        Kind: manifest.Kind,
        Available: manifest.Available,
        Size: manifest.Size?.Map(),
        ImageData: manifest.ImageData?.Map(),
        Descriptor: manifest.Descriptor_?.Map(),
        AttestationData: manifest.AttestationData?.Map()
    );

    internal static SizeInfo Map(this SizeMessage size) => new(
        Total: size.Total,
        Content: size.Content,
        Unpacked: size.Unpacked
    );

    internal static ImageData Map(this ImageDataMessage imageData) => new(
        Size: imageData.Size?.Map(),
        Platform: imageData.Platform?.Map(),
        Containers: imageData.Containers?.ToList() ?? []
    );

    internal static ImagePlatformDescriptor Map(this PlatformDescriptorMessage descriptor) => new(
        OS: descriptor.Os,
        Variant: descriptor.Variant,
        OSVersion: descriptor.OsVersion,
        Architecture: descriptor.Architecture,
        OSFeatures: descriptor.OsFeatures?.ToList() ?? []
    );

    internal static AttestationData Map(this AttestationDataMessage attestation) => new(
        For: attestation.For
    );

    internal static ImageHealthCheck Map(this HealthcheckMessage healthCheck) => new(
        Test: healthCheck.Test?.ToList() ?? [],
        Interval: healthCheck.Interval,
        Timeout: healthCheck.Timeout,
        Retries: healthCheck.Retries,
        StartPeriod: healthCheck.StartPeriod,
        StartInterval: healthCheck.StartInterval
    );

    internal static PullImageResult Map(this PullImageResponse pullResponse) => new
    (
        Id: pullResponse.Id,
        From: pullResponse.From,
        Stream: pullResponse.Stream,
        Status: pullResponse.Status,
        ErrorMessage: pullResponse.ErrorMessage,
        ProgressMessage: pullResponse.ProgressMessage,
        Progress: pullResponse.Progress?.Map(),
        Error: pullResponse.Error is not null ? new ImagePullError(pullResponse.Error.Code, pullResponse.Error.Message) : null
    );

    internal static ImagePullProgress Map(this JSONProgressReply jsonProgress) => new
    (
        Current: jsonProgress.Current,
        Total: jsonProgress.Total,
        Units: jsonProgress.Units,
        Start: jsonProgress.Start
    );

    internal static IReadOnlyList<ImageResult> Map(this IEnumerable<Hosting.DockerClient.ImageSummary> images) 
        => [.. images.Select(Map)];

    internal static ImageResult Map(this Hosting.DockerClient.ImageSummary image)
        => new
        (
            Id: image.Id,
            Size: image.Size,
            Containers: image.Containers,
            ParentId: image.ParentId,
            SharedSize: image.SharedSize,
            Created: image.Created,
            VirtualSize: image?.VirtualSize ?? 0,
            RepoTags: image.RepoTags?.ToList() ?? [],
            RepoDigests: image.RepoDigests?.ToList() ?? [],
            Labels: image.Labels?.ToDictionary() ?? []
        );

    internal static DeleteImageResult Map(this Hosting.DockerClient.Models.Images.DeleteImageResult response) 
        => new
        (
            Items: [.. response.Items.Select(item => new Domain.Contracts.Resources.Images.DeleteImageResponseItem(item.Result.ToDictionary()))]
        );

    internal static InspectImageResult Map(this Hosting.DockerClient.Models.Images.ImageInspectResult image)
        => new
        (
            Id: image.Id,
            Os: image.Os,
            Size: image.Size,
            Architecture: image.Architecture,
            Created: image.Created,
            Env: image.Env?.ToList() ?? [],
            Cmd: image.Cmd?.ToList() ?? [],
            RepoTags: image.RepoTags?.ToList() ?? [],
            Containers: image.Containers?.Map() ?? [],
            Volumes: image.Volumes?.ToList() ?? [],
            Labels: image.Labels,
            ExposedPorts: image.ExposedPorts?.ToList() ?? [],
            Layers: image.Layers?.Select(Map) ?? []

        );

    private static IEnumerable<Domain.Contracts.Resources.Images.ContainerImageResult> Map(this IEnumerable<Hosting.DockerClient.Models.Images.ContainerImage> containers)
    => containers.Select(Map);

    private static Domain.Contracts.Resources.Images.ContainerImageResult Map(this Hosting.DockerClient.Models.Images.ContainerImage container)
    => new(
        Id: container.Id,
        Name: container.Name,
        State: container.State.Map(),
        Volumes: container.Volumes?.ToList() ?? [],
        Networks: container.Networks ?? [],
        Ports: container.Ports?.Map() ?? []
    );


    private static ImageRootFs Map(this Hosting.DockerClient.RootFS rootFs) => new(
        Type: rootFs.Type,
        Layers: rootFs.Layers?.ToList() ?? []
    );

    private static ImageMetadata Map(this Hosting.DockerClient.Metadata metadata) => new(
        LastTagTime: metadata.LastTagTime
    );

    private static ImageConfig Map(this Hosting.DockerClient.ImageConfig imageConfig)
        => new(
            Tty: imageConfig.Tty ?? false,
            User: imageConfig.User,
            Image: imageConfig.Image,
            Hostname: imageConfig.Hostname,
            Domainname: imageConfig.Domainname,
            AttachStdin: imageConfig.AttachStdin ?? false,
            AttachStdout: imageConfig.AttachStdout ?? false,
            AttachStderr: imageConfig.AttachStderr ?? false,
            StopSignal: imageConfig.StopSignal,
            OpenStdin: imageConfig.OpenStdin ?? false,
            StdinOnce: imageConfig.StdinOnce ?? false,
            ArgsEscaped: imageConfig.ArgsEscaped ?? false,
            WorkingDir: imageConfig.WorkingDir,
            HealthCheck: imageConfig.Healthcheck is not null ? imageConfig.Healthcheck.Map() : null,
            Shell: imageConfig.Shell?.ToList() ?? [],
            Env: imageConfig.Env?.ToList() ?? [],
            Cmd: imageConfig.Cmd?.ToList() ?? [],
            OnBuild: imageConfig.OnBuild?.ToList() ?? [],
            EntryPoint: imageConfig.Entrypoint?.ToList() ?? [],
            Volumes: imageConfig.Volumes?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? [],
            Labels: imageConfig.Labels?.ToDictionary() ?? [],
            ExposedPorts: imageConfig.ExposedPorts?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? []
        );

    private static ImageDescriptor Map(this Hosting.DockerClient.OCIDescriptor ociDescriptor) => new(
        Size: ociDescriptor.Size ?? 0,
        Data: ociDescriptor.Data,
        Digest: ociDescriptor.Digest,
        MediaType: ociDescriptor.MediaType,
        ArtifactType: ociDescriptor.ArtifactType,
        Platform: ociDescriptor.Platform is not null ? ociDescriptor.Platform?.Map() : null,
        Urls: ociDescriptor.Urls?.Select(s => s.ToString()).ToList() ?? [],
        Annotations: ociDescriptor.Annotations?.ToDictionary() ?? []
    );
    private static ImageGraphicDriver Map(this Hosting.DockerClient.DriverData driverData) => new(
        Name: driverData.Name,
        Data: Map(driverData.Data)
    );

    private static ImageGraphDriverData Map(IDictionary<string, string> items)
    {
        string? merged = null; string? upper = null; string? work = null;
        items?.TryGetValue("MergedDir", out merged);
        items?.TryGetValue("UpperDir", out upper);
        items?.TryGetValue("WorkDir", out work);

        return new ImageGraphDriverData(merged ?? string.Empty, upper ?? string.Empty, work ?? string.Empty);
    }

    private static ImageManifest Map(this Hosting.DockerClient.ImageManifestSummary manifest) => new(
        Id: manifest.ID,
        Kind: manifest.Kind.ToString(),
        Available: manifest.Available,
        Size: manifest.Size?.Map(),
        ImageData: manifest.ImageData?.Map(),
        Descriptor: manifest.Descriptor?.Map(),
        AttestationData: manifest.AttestationData?.Map()
    );

    private static SizeInfo Map(this Hosting.DockerClient.Size size) => new(
        Total: size.Total,
        Content: size.Content,
        Unpacked: 0 // Unpacked size is not provided in the DockerClient image, defaulting to 0
    );

    private static ImageData Map(this Hosting.DockerClient.ImageData imageData) => new(
        Platform: imageData.Platform?.Map(),
        Containers: imageData.Containers?.ToList() ?? [],
        Size: imageData.Size?.Map()
    );

    private static SizeInfo Map(this Hosting.DockerClient.Size2 size) => new(
       Total: 0,
       Content: 0,
       Unpacked: size.Unpacked
    );

    private static ImagePlatformDescriptor Map(this Hosting.DockerClient.OCIPlatform oci) => new(
        OS: oci.Os,
        Variant: oci.Variant,
        OSVersion: oci.OsVersion,
        Architecture: oci.Architecture,
        OSFeatures: oci.OsFeatures?.ToList() ?? []
    );

    private static AttestationData Map(this Hosting.DockerClient.AttestationData attestation) => new(
        For: attestation.For
    );

    private static ImageHealthCheck Map(this Hosting.DockerClient.HealthConfig healthConfig) => new(
        Test: healthConfig.Test?.ToList() ?? [],
        Interval: healthConfig.Interval,
        Timeout: healthConfig.Timeout,
        Retries: healthConfig.Retries,
        StartPeriod: healthConfig.StartPeriod,
        StartInterval: healthConfig.StartInterval
    );

    internal static PullImageResult Map(this Hosting.DockerClient.HttpClient.JSONMessage jsonMessage) => new
    (
        Id: jsonMessage.ID,
        From: jsonMessage.From,
        Stream: jsonMessage.Stream,
        Status: jsonMessage.Status,
        ErrorMessage: jsonMessage.ErrorMessage,
        ProgressMessage: jsonMessage.ProgressMessage,
        Progress: jsonMessage.Progress?.Map(),
        Error: jsonMessage.Error is not null ? new ImagePullError(jsonMessage.Error.Code, jsonMessage.Error.Message) : null
    );

    internal static ImagePullProgress Map(this Hosting.DockerClient.HttpClient.JSONProgress jsonProgress) => new
    (
        Current: jsonProgress.Current,
        Total: jsonProgress.Total,
        Units: jsonProgress.Units,
        Start: jsonProgress.Start
    );
    internal static IEnumerable<Domain.Contracts.Resources.Images.HistoryImageResult> Map(this IEnumerable<Citadel.Agent.Images.V1.HistoryImageResult> items) => items.Select(Map);

    internal static IEnumerable<Domain.Contracts.Resources.Images.HistoryImageResult> Map(this IEnumerable<Hosting.DockerClient.HistoryResponseItem> items) => items.Select(Map);

    internal static Domain.Contracts.Resources.Images.HistoryImageResult Map(this Hosting.DockerClient.HistoryResponseItem item) => new (
        Id: item.Id, Created : item.Created, CreatedBy: item.CreatedBy, Size: item.Size, Comment: item.Comment);

    internal static Domain.Contracts.Resources.Images.HistoryImageResult Map(this Citadel.Agent.Images.V1.HistoryImageResult item) => new(
        Id: item.Id, Created: item.Created, CreatedBy: item.CreatedBy, Size: item.Size, Comment: item.Comment);

    internal static RunImageInfoResult Map(this Hosting.DockerClient.Models.Images.RunImageInfoResult info) =>
        new(Volumes: info.Volumes, Networks: info.Networks, ExposedPorts: info.ExposedPorts, 0, 0);

    internal static List<Domain.Contracts.Resources.Images.HistoryImageResult> Map(this HistoryImageResponse histories) 
        => [.. histories.Items.Select(Map)];

    internal static Domain.Contracts.Resources.Images.HistoryImageResult Map(this HistoryImageItemResponse item) 
        => new (
            Id: item.Id,
            Created: item.Created,
            CreatedBy: item.CreatedBy,
            Size: item.Size,
            Comment: item.Comment
        );
}
