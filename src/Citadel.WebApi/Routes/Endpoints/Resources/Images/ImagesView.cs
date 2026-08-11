using Application.Permissions;
using Application.Features.Images.Queries;
using Domain;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record ImageView(
    Guid Id,
    IEnumerable<string> Tags,
    string Name,
    string DockerImageId,
    double Size,
    bool IsInUse,
    Guid PlatformId,
    DateTime CreatedAt,
    ResourceControlState ControlState,
    DateTime? UpdatedAt = null,
    Guid? RegistryId = null,
    RegistryView? Registry = null,
    ImageCapabilities? Capabilities = null,
    IReadOnlyList<string>? RepoDigests = null,
    string? ContentIdentity = null,
    string? DockerNodeId = null,
    string? NodeHostname = null,
    bool IsStale = false,
    string? StaleReason = null
);

public sealed record ImagesView(IEnumerable<ImageView> Images, ImageCapabilities Capabilities)
{
    internal static async Task<ImagesView> Map(
        IEnumerable<LocalImageInventoryItem> images,
        IPermissionEvaluator permissionEvaluator)
    {
        var list = images as LocalImageInventoryItem[] ?? [.. images];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Platform);
        if (list.Length == 0)
            return new ImagesView([], CapabilityMapper.ToImageCapabilities(resourcesPerms));

        var platformId = list[0].StandaloneImage?.PlatformId
                         ?? list[0].NodeImage?.PlatformId
                         ?? Guid.Empty;
        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);
        var capabilities = CapabilityMapper.ToImageCapabilities(
            meta == default ? PermissionMetadata.Empty : meta);
        return new ImagesView(
            list.Select(item => Map(item) with { Capabilities = capabilities }).ToArray(),
            CapabilityMapper.ToImageCapabilities(resourcesPerms));
    }

    private static ImageView Map(LocalImageInventoryItem item)
    {
        if (item.StandaloneImage is { } image)
            return Map(image);

        var projection = item.NodeImage
                         ?? throw new InvalidDataException("An Image inventory item has no resource.");
        return Map(projection);
    }

    internal static ImageView Map(SwarmNodeImageProjection projection)
    {
        var resource = projection.Resource;
        var name = resource.RepoTags?.FirstOrDefault()?.Split(':')[0]
                   ?? resource.RepoDigests?.FirstOrDefault()?.Split('@')[0]
                   ?? resource.Id;
        return new ImageView(
            projection.Id,
            resource.RepoTags ?? [],
            name,
            resource.Id,
            resource.Size,
            resource.Containers > 0,
            projection.PlatformId,
            DateTimeOffset.FromUnixTimeSeconds(resource.Created).UtcDateTime,
            ResourceControlState.Idle,
            RepoDigests: resource.RepoDigests ?? [],
            ContentIdentity: projection.ContentIdentity,
            DockerNodeId: projection.DockerNodeId,
            NodeHostname: projection.NodeHostname,
            IsStale: projection.IsStale,
            StaleReason: projection.StaleReason);
    }

    internal static async Task<ImagesView> Map(IEnumerable<Image> images, IPermissionEvaluator permissionEvaluator)
    {
        var list = images as Image[] ?? [.. images];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Platform);
        if (list.Length == 0)
            return new ImagesView([], CapabilityMapper.ToImageCapabilities(resourcesPerms));

        // All images belong to the same platform capability scope.
        // Resolve once instead of N times.
        var platformId = list[0].PlatformId;

        var meta = await permissionEvaluator.EvaluateAsync(platformId, ResourceType.Platform);

        var capabilities = CapabilityMapper.ToImageCapabilities(meta == default ? PermissionMetadata.Empty : meta);

        var views = new ImageView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            views[i] = Map(list[i]) with
            {
                Capabilities = capabilities
            };
        }

        return new ImagesView(views, CapabilityMapper.ToImageCapabilities(resourcesPerms));
    }

    internal static ImagesView Map(IEnumerable<Image> images) => new(images?.Select(Map) ?? [], ImageCapabilities.Empty);
    internal static ImageView Map(Image image)
        => new(
            Id: image.Id,
            Tags: image.Tags,
            Name: image.Name,
            DockerImageId: image.DockerImageId,
            Size: image.Size,
            IsInUse: image.Containers > 0,
            PlatformId: image.PlatformId,
            CreatedAt: image.CreatedAt,
            UpdatedAt: image.UpdatedAt,
            RegistryId: image.RegistryId,
            ControlState: image.ControlState,
            Registry: image.Registry is not null ? RegistryView.Map(image.Registry) : null
        );

    internal static async Task<ImageView> Map(Image image, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(image.PlatformId, ResourceType.Platform);
        return Map(image) with
        {
            Capabilities = CapabilityMapper.ToImageCapabilities(permissions)
        };
    }
}
