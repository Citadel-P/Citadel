using Infrastructure.Entities;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistriesView(IEnumerable<RegistryView> Registries)
{
    internal static RegistriesView Map(IEnumerable<Registry> registries) => new RegistriesView(registries.Select(RegistryView.Map).ToList());
}
