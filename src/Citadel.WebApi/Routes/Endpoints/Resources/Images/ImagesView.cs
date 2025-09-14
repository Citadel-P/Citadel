using Application.Mappers;
using Domain.Contracts.Resources.Images;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record ImageView(
    string Id,
    string Tag,
    string? Name,
    long Created,
    string ParentId,
    IEnumerable<string> RepoDigests,
    IEnumerable<string> RepoTags,
    long SharedSize,
    double Size,
    double VirtualSize,
    bool IsInUse,
    IDictionary<string, string> Labels);

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
    internal static ImagesView Map(IEnumerable<ImageResult> images) => new (images.Select(Map));
    internal static ImageView Map(ImageResult image) 
        => new (
            Id: image.Id,
            Tag: image.GetTag(),
            Name: image.GetName(),
            Size: image.Size,
            Created: image.Created,
            ParentId: image.ParentId,
            SharedSize: image.SharedSize,
            VirtualSize: image.VirtualSize,
            IsInUse: image.Containers > 0,
            RepoTags: image.RepoTags ?? [],
            RepoDigests: image.RepoDigests ?? [],
            Labels: image.Labels?.ToDictionary() ?? []
        );
}
