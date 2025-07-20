namespace Infrastructure.Persistence.Dtos;

internal sealed record RegistryDto(
    string Id, //Guid
    string Name,
    string Url,
    string Created, // DateTime
    string Type, // RegistryType
    string Configuration // RegistryConfigurationBase
    )
{
    public RegistryDto(): this(string.Empty, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty)
    {
    }
}