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
    bool? IsUpToDate = null,
    DateTime? UpdatedAt = null,
    Guid? RegistryId = null,
    string? RegistryName = null,
    string? RegistryHost = null,
    DateTime? RegistryCreated = null
    );
