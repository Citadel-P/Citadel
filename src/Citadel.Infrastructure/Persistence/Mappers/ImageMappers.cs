using Domain;
using Domain.Entities;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class ImageMappers
{
    internal static IEnumerable<Image> ToDomain(this IEnumerable<ImageDto> images)
        => images.Select(ToDomain);

    internal static Image ToDomain(this ImageDto image)
        => Image.FromPersistence(
                id: image.Id,
                name: image.Name,
                tags: string.IsNullOrEmpty(image?.Tags) ? [] : JsonSerializer.Deserialize(image.Tags, ImagTagsContext.Default.IEnumerableString),
                dockerImageId: image.DockerImageId,
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
                    description: null,
                    status: Enum.TryParse<RegistryStatus>(image?.RegistryStatus, out var status) ? status : RegistryStatus.Active,
                    registryHost: image?.RegistryHost ?? "",
                    createdAt: image?.RegistryCreatedAt ?? DateTime.MinValue,
                    createdByActorId: image?.RegistryCreatedByActorId ?? Guid.Empty,
                    configuration: null)
            );
}
