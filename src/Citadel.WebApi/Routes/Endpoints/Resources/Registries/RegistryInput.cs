using Application.Features.Registries.Commands;
using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryInput(
    string Name, 
    string RegistryHost,
    RegistryStatus Status,
    RegistryConfigurationBase Configuration,
    string? Description = null
    )
{
    internal CreateRegistry ToCommand() => new(Name, GetRegistryHost(), Status, Configuration, Description);

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