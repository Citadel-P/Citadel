using Application.Features.Registries.Commands;
using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record CreateRegistryInput(
    string Name,
    string RegistryHost,
    RegistryStatus Status,
    RegistryConfiguration Configuration,
    string? Description = null,
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal CreateRegistry ToCommand() => new(Name, GetRegistryHost(), Status, Configuration, Description, TagIds);

    private string GetRegistryHost()
    {
        if (Configuration is DockerHubRegistry)
        {
            return "docker.io";
        }
        if (Configuration is GitHubRegistry)
        {
            return "ghcr.io";
        }

        return RegistryHost;
    }
}
