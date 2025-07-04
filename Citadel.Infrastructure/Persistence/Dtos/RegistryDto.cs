using Domain;
using Domain.Entities.Registries;

namespace Infrastructure.Persistence.Dtos;

internal sealed class RegistryDto
{
    public Guid Id { get; set; }
    public string Name { get; set; } = default!;
    public string Url { get; set; } = default!;
    public DateTime Created { get; set; }
    public RegistryType Type { get; set; }
    public RegistryConfigurationBase Configuration { get; set; } = default!;
}