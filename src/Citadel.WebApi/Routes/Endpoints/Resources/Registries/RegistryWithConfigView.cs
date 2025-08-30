using Domain;
using Domain.Entities;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryWithConfigView(Guid Id, string Name, string Url, RegistryType Type, DateTime Created, RegistryConfigurationBase? Configuration)
{
    internal static RegistryWithConfigView Map(Registry registry) => new(registry.Id, registry.Name, registry.Url, registry.Type, registry.Created, registry.Configuration);
}