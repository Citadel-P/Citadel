using Domain.Contracts.Resources.Images;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record ImageView(
    string Id,
    long Created,
    string ParentId,
    IEnumerable<string> RepoDigests,
    IEnumerable<string> RepoTags,
    long SharedSize,
    double Size,
    double VirtualSize,
    bool IsInUse,
    IDictionary<string, string> Labels)
{
    public string? Name => GetImageName();
    public string Tag => GetTag();

    private string? GetImageName()
    {
        if (RepoTags.Any()) return RepoTags.First().Split(":").FirstOrDefault();
        else if (RepoDigests.Any()) return RepoDigests.FirstOrDefault()?.Split("@").FirstOrDefault();
        else if (Labels.Any()) return Labels.TryGetValue("org.opencontainers.image.title", out var label) ? label : null;
        else return Id;
    }

    private string GetTag()
        => RepoTags.FirstOrDefault()?.Split(":").LastOrDefault() ?? "none";
}

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
    internal static ImagesView Map(IEnumerable<DockerImage> images) => new (images.Select(Map));
    internal static ImageView Map(DockerImage image) 
        => new (
            Id: image.Id,
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
