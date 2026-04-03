using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record PatchRegistryInput(
    string RegistryHost,
    RegistryStatus Status,
    RegistryConfiguration Configuration);
