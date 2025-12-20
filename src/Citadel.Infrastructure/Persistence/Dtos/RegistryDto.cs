namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    Guid Id,
    string Name,
    string RegistryHost,
    string Status,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string Configuration, // RegistryConfigurationBase
    string? Description = null
    );