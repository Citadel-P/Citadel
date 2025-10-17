using Domain.Contracts.Resources.Images;
using Domain.Entities;

namespace Application.Mappers;

internal static class ImageMapper
{
    internal static string? GetName(this ImageResult image)
    {
        if (image == null) return null;

        if (image.RepoTags != null && image.RepoTags.Any()) return image.RepoTags.First().Split(":").FirstOrDefault();
        else if (image.RepoDigests != null && image.RepoDigests.Any()) return image.RepoDigests.FirstOrDefault()?.Split("@").FirstOrDefault();
        else if (image.Labels != null && image.Labels.Any()) return image.Labels.TryGetValue("org.opencontainers.image.title", out var label) ? label : null;
        else return image.Id;
    }

    internal static string GetTag(this ImageResult image)
    {
        return image.RepoTags?.FirstOrDefault()?.Split(":").LastOrDefault() ?? "none";
    }

    internal static IEnumerable<Image> Map(this IEnumerable<ImageResult> images, Guid platformId, Registry? registry = null)
        => images.Select(s => s.Map(platformId, registry));

    internal static Image Map(this ImageResult image, Guid platformId, Registry? registry = null)
        => new (
            name: image.GetName() ?? string.Empty,
            tag: image.GetTag(),
            dockerImageId: image.Id,
            size: image.Size,
            platformId: platformId,
            registryId: registry?.Id == Guid.Empty ? null : registry?.Id,
            containers: image.Containers,
            createdAt: DateTimeOffset.FromUnixTimeSeconds(image.Created).UtcDateTime,
            registry: registry
            );
}
