using Domain;
using Domain.Entities;
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
    RegistryView? Registry = null);

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
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
}
