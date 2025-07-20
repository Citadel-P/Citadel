using Domain;
using Domain.Contracts.Resources.Registries;

namespace WebApi.Routes.Endpoints.Resources.Images;

/// <summary>
/// 
/// </summary>
/// <param name="Id"></param>
/// <param name="Name">Name of the tag</param>
/// <param name="Image"></param>
/// <param name="LastUpdated"></param>
/// <param name="FullSize">Compressed size (sum of all layers) of the tagged image</param>
/// <param name="Status">Whether a tag has been pushed to or pulled in the past month</param>
/// <param name="LastPulled">Datetime of last pull</param>
/// <param name=""></param>
public sealed record DockerHubTagView(int Id, string Name, DockerHubImageView? Image, string LastUpdated, int FullSize, DockerHubTagStatus Status, string LastPulled);

/// <summary>
/// 
/// </summary>
/// <param name="Architecture">CPU architecture</param>
/// <param name="Digest">Image digest</param>
/// <param name="Os">Operating system</param>
/// <param name="Size">Size of the image</param>
/// <param name="Status">Status of the image</param>
/// <param name="LastPulled">Datetime of last pull</param>
public sealed record DockerHubImageView(string Architecture, string Digest, string Os, int Size, DockerHubImageStatus Status, string LastPulled);

internal static partial class Mapper
{
    internal static IEnumerable<DockerHubTagView> Map(IEnumerable<DockerHubTag> tags) => tags.Select(Map);

    internal static DockerHubTagView Map(this DockerHubTag tag) => new(
        Id: tag.Id,
        Name: tag.Name,
        Status: tag.Status,
        FullSize: tag.FullSize,
        LastUpdated: tag.LastUpdated,
        LastPulled: tag.TagLastPulled,
        Image: tag.Images.FirstOrDefault()?.Map());

    internal static DockerHubImageView Map(this DockerHubImage image) => new(
        image.Architecture,
        image.Digest,
        image.Os,
        image.Size,
        image.Status,
        image.LastPulled);
}
