namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    Guid Id,
    string Name,
    string Url,
    string Created, // DateTime
    string Type, // RegistryType
    string Configuration // RegistryConfigurationBase
    );