using System.Text.Json;
using Infrastructure;
using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryView(Guid Id, string Name, string Url, RegistryDiscriminator Discriminator, DateTime Created, IRegistryConfiguration Configuration)
{
    internal static RegistryView Map(Registry registry) => new(registry.Id, registry.Name, registry.Url, registry.Discriminator, registry.Created,
        JsonSerializer.Deserialize<IRegistryConfiguration>(registry.Configuration, Hosting.Common.Helpers.CommonJsonOptions));
}