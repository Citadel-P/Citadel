using Application.Permissions;
using Domain;
using Domain.Entities;
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
    ImageCapabilities? Capabilities = null
);

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
    internal static async Task<ImagesView> Map(IEnumerable<Image> images, IPermissionEvaluator permissionEvaluator)
    {
        var list = images as Image[] ?? [.. images];

        if (list.Length == 0)
            return new ImagesView([]);

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

        return new ImagesView(views);
    }

    internal static ImagesView Map(IEnumerable<Image> images) => new(images?.Select(Map) ?? []);
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
