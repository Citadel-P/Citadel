using Domain;
using Domain.Entities;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryWithConfigView(
    Guid Id, 
    string Name,
    string RegistryHost,
    RegistryStatus Status,
    string Description,
    DateTime Created,
    RegistryConfigurationBase? Configuration)
{
    internal static RegistryWithConfigView Map(Registry registry) => new(registry.Id, registry.Name, registry.RegistryHost, registry.Status, registry.Description ?? "", registry.CreatedAt, registry.Configuration);
}