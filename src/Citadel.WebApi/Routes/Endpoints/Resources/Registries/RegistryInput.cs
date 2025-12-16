using Application.Features.Registries.Commands;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryInput(
    string Name, 
    string RegistryHost,
    RegistryConfigurationBase Configuration,
    string? Description = null
    )
{
    internal CreateRegistry ToCommand() => new(Name, GetRegistryHost(), Configuration, Description);

    private string GetRegistryHost()
    {
        if (Configuration is DockerHubRegistry)
        {
            return "docker.io";
        }
        else if (Configuration is GitHubRegistry)
        {
            return "ghcr.io";
        }

        return RegistryHost;
    }
}