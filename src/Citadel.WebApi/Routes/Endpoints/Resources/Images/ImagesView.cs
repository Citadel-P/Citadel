using Domain.Entities;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record ImageView(
    Guid Id,
    string Tag,
    string Name,
    string ImageId,
    double Size,
    bool IsInUse,
    Guid PlatformId,
    DateTime CreatedAt,
    bool? IsUpToDate = null,
    DateTime? UpdatedAt = null,
    Guid? RegistryId = null,
    RegistryView? Registry = null);

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
    internal static ImagesView Map(IEnumerable<Image> images) => new(images.Select(Map));
    internal static ImageView Map(Image image)
        => new(
            Id: image.Id,
            Tag: image.Tag,
            Name: image.Name,
            ImageId: image.ImageId,
            Size: image.Size,
            IsInUse: image.Containers > 0,
            PlatformId: image.PlatformId,
            CreatedAt: image.CreatedAt,
            IsUpToDate: image.IsUpToDate,
            UpdatedAt: image.UpdatedAt,
            RegistryId: image.RegistryId,
            Registry: image.Registry is not null ? RegistryView.Map(image.Registry) : null
        );
}
