using System;
using System.Linq;
using Citadel.Agent.Images.V1;
using Domain.Contracts.Resources.Images;
using static Citadel.Agent.Images.V1.ConfigMessage.Types;

namespace Infrastructure.Connectors.Mappers;

internal static class ImageMapper
{
    public static List<DockerImage> Map(this IEnumerable<ImageReply> images)
        => [.. images.Select(Map)];

    public static DockerImage Map(this ImageReply image)
        => new 
        (
            Id: image.Id,
            Size: image.Size,
            Containers: image.Containers,
            ParentId: image.ParentId,
            SharedSize: image.SharedSize,
            Created: image.Created,
            VirtualSize: image.VirtualSize,
            RepoTags: image.RepoTags?.ToList(),
            RepoDigests: image.RepoDigests?.ToList(),
            Labels: image.Labels?.ToDictionary()
        );

    public static DeleteImageResult Map(this DeleteImageResponse response) 
        => new
        (
            Items: [.. response.Items.Select(item => new Domain.Contracts.Resources.Images.DeleteImageResponseItem(item.Result.ToDictionary()))]
        );

    public static InspectImageResult Map(this InspectImageResponse response)
        => new
        (
            Id: response.Id,
            Author: response.Author,
            Parent: response.Parent,
            Comment: response.Comment,
            Created: response.Created,
            DockerVersion: response.DockerVersion,
            Architecture: response.Architecture,
            OSVersion: response.OsVersion,
            VirtualSize: response.VirtualSize,
            Variant: response.Variant,
            OS: response.Os,
            Size: response.Size,
            RootFS: response.RootFS?.Map(),
            Metadata: response.Metadata?.Map(),
            Config: response.Config?.Map(),
            Descriptor: response.Descriptor_?.Map(),
            GraphDriver: response.GraphDriver?.Map(),
            Manifests: response.Manifests.Select(m => m.Map())?.ToList() ?? [],
            RepoTags: response.RepoTags?.ToList() ?? [],
            RepoDigests: response.RepoDigests?.ToList() ?? []
        );
    public static ImageRootFs Map(this RootFSMessage reply) => new(
        Type: reply.Type,
        Layers: reply.Layers?.ToList() ?? []
    );

    public static ImageMetadata Map(this MetadataMessage reply) => new(
        LastTagTime: reply.LastTagTime
    );

    public static ImageConfig Map(this ConfigMessage reply) 
        => new(
            Tty: reply.Tty,
            User: reply.User,
            Image: reply.Image,
            Hostname: reply.Hostname,
            Domainname: reply.Domainname,
            AttachStdin: reply.AttachStdin,
            AttachStdout: reply.AttachStdout,
            AttachStderr: reply.AttachStderr,
            StopSignal: reply.StopSignal,
            OpenStdin: reply.OpenStdin,
            StdinOnce: reply.StdinOnce,
            ArgsEscaped: reply.ArgsEscaped,
            WorkingDir: reply.WorkingDir,
            HealthCheck: reply.Healthcheck is not null ? reply.Healthcheck.Map() : null,
            Shell: reply.Shell?.ToList() ?? [],
            Env: reply.Env?.ToList() ?? [],
            Cmd: reply.Cmd?.ToList() ?? [],
            OnBuild: reply.OnBuild?.ToList() ?? [],
            EntryPoint: reply.Entrypoint?.ToList() ?? [],
            Volumes: reply.Volumes?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? [],
            Labels: reply.Labels?.ToDictionary() ?? [],
            ExposedPorts: reply.ExposedPorts?.ToDictionary(x => x.Key, x => new Domain.Contracts.Resources.Images.Empty()) ?? []
        );

    public static ImageDescriptor Map(this DescriptorMessage reply) => new(
        Size: reply.Size,
        Data: reply.Data,
        Digest: reply.Digest,
        MediaType: reply.MediaType,
        ArtifactType: reply.ArtifactType,
        Platform: reply.Platform is not null ? reply.Platform.Map() : null,
        Urls: reply.Urls?.ToList() ?? [],
        Annotations: reply.Annotations?.ToDictionary() ?? []
    );

    public static ImageGraphicDriver Map(this GraphDriverMessage reply) => new(
        Name: reply.Name,
        Data: reply.Data?.Map()
    );

    public static ImageGraphDriverData Map(this GraphDriverDataMessage reply) => new(
        MergedDir: reply.MergedDir,
        UpperDir: reply.UpperDir,
        WorkDir: reply.WorkDir
    );

    public static ImageManifest Map(this ManifestMessage reply) => new(
        Id: reply.Id,
        Kind: reply.Kind,
        Available: reply.Available,
        Size: reply.Size?.Map(),
        ImageData: reply.ImageData?.Map(),
        Descriptor: reply.Descriptor_?.Map(),
        AttestationData: reply.AttestationData?.Map()
    );

    // Add mapping methods for nested types as needed, e.g.:
    public static SizeInfo Map(this SizeMessage reply) => new(
        Total: reply.Total,
        Content: reply.Content,
        Unpacked: reply.Unpacked
    );

    public static ImageData Map(this ImageDataMessage reply) => new(
        Platform: reply.Platform?.Map(),
        Containers: reply.Containers?.ToList() ?? [],
        Size: reply.Size?.Map()
    );

    public static ImagePlatformDescriptor Map(this PlatformDescriptorMessage reply) => new(
        OS: reply.Os,
        Variant: reply.Variant,
        OSVersion: reply.OsVersion,
        Architecture: reply.Architecture,
        OSFeatures: reply.OsFeatures?.ToList() ?? []
    );

    public static AttestationData Map(this AttestationDataMessage reply) => new(
        For: reply.For
    );

    public static ImageHealthCheck Map(this HealthcheckMessage reply) => new(
        Test: reply.Test?.ToList() ?? [],
        Interval: reply.Interval,
        Timeout: reply.Timeout,
        Retries: reply.Retries,
        StartPeriod: reply.StartPeriod,
        StartInterval: reply.StartInterval
    );

    public static PullImageResult Map(this PullImageResponse response) => new
    (
        Id: response.Id,
        From: response.From,
        Stream: response.Stream,
        Status: response.Status,
        ErrorMessage: response.ErrorMessage,
        ProgressMessage: response.ProgressMessage,
        Progress: response.Progress?.Map(),
        Error: response.Error is not null ? new ImagePullError(response.Error.Code, response.Error.Message) : null
    );

    public static ImagePullProgress Map(this JSONProgressReply reply) => new
    (
        Current: reply.Current,
        Total: reply.Total,
        Units: reply.Units,
        Start: reply.Start
    );
}
