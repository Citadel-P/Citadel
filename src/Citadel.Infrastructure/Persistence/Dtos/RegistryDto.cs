namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    Guid Id,
    string Name,
    string RegistryHost,
    string CreatedAt, // DateTime
    Guid CreatedByActorId,
    string Configuration, // RegistryConfigurationBase
    string? Description = null
    );