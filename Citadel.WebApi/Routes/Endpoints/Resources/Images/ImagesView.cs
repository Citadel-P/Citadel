using Agent.Server.Images;

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
    IDictionary<string, string> Labels)
{
    public string Name => GetImageName();
    public string Tag => GetTag();

    private string GetImageName()
    {
        if (RepoTags.Any()) return RepoTags.First().Split(":").FirstOrDefault();
        else if (RepoDigests.Any()) return RepoDigests.FirstOrDefault()?.Split("@").FirstOrDefault();
        else if (Labels.Any()) return Labels["org.opencontainers.image.title"];
        else return Id;
    }

    private string GetTag()
        => RepoTags.FirstOrDefault()?.Split(":").LastOrDefault() ?? "none";
}

public sealed record ImagesView(IEnumerable<ImageView> Images)
{
    internal static ImagesView Map(IEnumerable<ImageReply> replies) => new (replies.Select(Map));
    internal static ImageView Map(ImageReply reply) 
        => new (
            reply.Id,
            reply.Created,
            reply.ParentId,
            reply.RepoDigests,
            reply.RepoTags,
            reply.SharedSize,
            reply.Size,
            reply.VirtualSize,
            reply.Labels
        );
}
