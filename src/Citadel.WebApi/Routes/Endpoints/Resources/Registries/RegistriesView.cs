using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistriesView(IEnumerable<RegistryView> Registries)
{
    internal static RegistriesView Map(IEnumerable<Registry> registries) => new([.. registries.Select(RegistryView.Map)]);
}
