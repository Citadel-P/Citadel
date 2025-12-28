namespace Infrastructure.Persistence.Dtos;

internal sealed record ImageDto(
    Guid Id,
    string Name,
    string Tags, // List<string>
    string DockerImageId,
    double Size,
    int Containers,
    Guid PlatformId,
    DateTime CreatedAt,
    DateTime? UpdatedAt = null,
    Guid? RegistryId = null,
    string? RegistryName = null,
    string? RegistryHost = null,
    string? RegistryStatus = null,
    DateTime? RegistryCreatedAt = null,
    Guid? RegistryCreatedByActorId = null,
    string? RegistryConfiguration = null
    );
