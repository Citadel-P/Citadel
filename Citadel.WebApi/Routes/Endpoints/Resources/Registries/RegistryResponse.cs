using Infrastructure;
using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryResponse(Guid Id, string Name, string Url, RegistryDiscriminator Discriminator, DateTime Created, IRegistryConfiguration Configuration)
{
    internal static RegistryResponse Map(Registry registry) => new(registry.Id, registry.Name, registry.Url, registry.Discriminator, registry.Created, null);
}