namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    Guid Id,
    string Name,
    string RegistryHost,
    string Created, // DateTime
    string Configuration // RegistryConfigurationBase
    );