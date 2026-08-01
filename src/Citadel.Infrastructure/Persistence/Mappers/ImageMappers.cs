using Domain;
using Domain.Entities;
using Domain.Entities.Registries;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class ImageMappers
{
    internal static IEnumerable<Image> ToDomain(this IEnumerable<ImageDto> images)
        => images.Select(ToDomain);

    internal static Image ToDomain(this ImageDto image)
    {
        var tags = string.IsNullOrEmpty(image.Tags)
            ? []
            : JsonSerializer.Deserialize(image.Tags, ImagTagsContext.Default.IEnumerableString) ?? [];

        Registry? registry = null;
        if (image.RegistryId is { } registryId)
        {
            var configuration = string.IsNullOrWhiteSpace(image.RegistryConfiguration)
                ? throw new InvalidDataException($"Registry {registryId} has no configuration.")
                : JsonSerializer.Deserialize(
                    image.RegistryConfiguration,
                    RegistryJsonContext.Default.RegistryConfiguration)
                  ?? throw new InvalidDataException($"Registry {registryId} has an invalid configuration.");

            registry = Registry.FromPersistence(
                id: registryId,
                name: image.RegistryName ?? string.Empty,
                description: null,
                status: Enum.TryParse<RegistryStatus>(image.RegistryStatus, out var status)
                    ? status
                    : RegistryStatus.Active,
                registryHost: image.RegistryHost ?? string.Empty,
                createdAt: image.RegistryCreatedAt ?? DateTime.MinValue,
                createdByActorId: image.RegistryCreatedByActorId ?? Guid.Empty,
                configuration: configuration);
        }

        return Image.FromPersistence(
            id: image.Id,
            name: image.Name,
            tags: tags,
            dockerImageId: image.DockerImageId,
            size: image.Size,
            containers: image.Containers,
            platformId: image.PlatformId,
            createdAt: image.CreatedAt,
            updatedAt: image.UpdatedAt,
            registryId: image.RegistryId,
            rowVersion: image.RowVersion,
            controlStartedAt: image.ControlStartedAt,
            controlState: image.ControlState != null
                ? Enum.Parse<ResourceControlState>(image.ControlState)
                : ResourceControlState.Idle,
            registry: registry);
    }
}
