namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    Guid Id,
    string Name,
    string RegistryHost,
    string Status,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string Configuration, // RegistryConfiguration
    string? Description = null,
    string? TagsJson = null
    );
