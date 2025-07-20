using Application.Features.Registries.Commands;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record DeleteRegistriesInput(IEnumerable<Guid> Ids)
{
    internal DeleteRegistries ToCommand() => new (Ids);
}
