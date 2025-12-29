using Domain;
using Domain.Entities;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryConfigView(
    Guid Id, 
    string Name,
    string RegistryHost,
    RegistryStatus Status,
    string Description,
    RegistryConfigurationBase? Configuration)
{
    internal static RegistryConfigView Map(Registry registry) => new(registry.Id, registry.Name, registry.RegistryHost, registry.Status, registry.Description ?? "", registry.Configuration);
}