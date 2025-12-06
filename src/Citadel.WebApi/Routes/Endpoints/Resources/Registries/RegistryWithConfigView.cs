using Domain;
using Domain.Entities;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryWithConfigView(Guid Id, string Name, string RegistryHost, DateTime Created, RegistryConfigurationBase? Configuration)
{
    internal static RegistryWithConfigView Map(Registry registry) => new(registry.Id, registry.Name, registry.RegistryHost, registry.Created, registry.Configuration);
}