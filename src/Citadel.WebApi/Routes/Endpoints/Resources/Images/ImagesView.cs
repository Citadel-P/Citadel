using Domain.Entities;

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
    Guid? RegistryId = null);

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
            IsInUse: image.IsInUse,
            PlatformId: image.PlatformId,
            CreatedAt: image.CreatedAt,
            IsUpToDate: image.IsUpToDate,
            UpdatedAt: image.UpdatedAt,
            RegistryId: image.RegistryId
        );

    //internal static ImagesView Map(IEnumerable<ImageResult> images) => new (images.Select(Map));
    //internal static ImageView Map(ImageResult image) 
    //    => new (
    //        Id: image.Id,
    //        Tag: image.GetTag(),
    //        Name: image.GetName(),
    //        Size: image.Size,
    //        Created: image.Created,
    //        ParentId: image.ParentId,
    //        SharedSize: image.SharedSize,
    //        VirtualSize: image.VirtualSize,
    //        IsInUse: image.Containers > 0,
    //        RepoTags: image.RepoTags ?? [],
    //        RepoDigests: image.RepoDigests ?? [],
    //        Labels: image.Labels?.ToDictionary() ?? []
    //    );
}
