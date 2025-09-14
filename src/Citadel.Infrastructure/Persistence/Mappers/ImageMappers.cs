using Domain.Entities;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class ImageMappers
{
    internal static IEnumerable<Image> ToDomain(this IEnumerable<ImageDto> images)
        => images.Select(ToDomain);

    internal static Image ToDomain(this ImageDto image)
        => Image.FromPersistence(
                id: image.Id,
                name: image.Name,
                tag: image.Tag,
                imageId: image.ImageId,
                size: image.Size,
                isInUse: image.IsInUse,
                platformId: image.PlatformId,
                createdAt: image.CreatedAt,
                isUpToDate: image.IsUpToDate,
                updatedAt: image.UpdatedAt,
                registryId: image.RegistryId
            );
}
