using Domain;
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
                containers: image.Containers,
                platformId: image.PlatformId,
                createdAt: image.CreatedAt,
                isUpToDate: image.IsUpToDate,
                updatedAt: image.UpdatedAt,
                registryId: image.RegistryId,
                registry: Registry.FromPersistence(
                    id: image?.RegistryId ?? Guid.Empty,
                    name: image?.RegistryName ?? "",
                    type: image?.RegistryType == null ? RegistryType.DockerHub : Enum.Parse <RegistryType>(image.RegistryType),
                    url: image?.RegistryUrl ?? "",
                    created: image?.RegistryCreated ?? DateTime.MinValue,
                    configuration: null)
            );
}
