using System.Text.Json.Serialization;
using Application.Features.Registries.Commands;
using Domain;
using Domain.Entities.Registries;

namespace WebApi.Routes.Endpoints.Resources.Registries;

public sealed record RegistryInput(
    string Name, 
    string RegistryHost,
    [property: JsonConverter(typeof(Citadel.GeneratedConverters.SafeRegistryTypeConverter))]
    RegistryType Type,
    RegistryConfigurationBase Configuration
    )
{
    internal CreateRegistry ToCommand() => new(Name, GetRegistryHost(), Type, Configuration);

    private string GetRegistryHost()
    {
        if (Type == RegistryType.DockerHub)
        {
            return "docker.io";
        }
        else if (Type == RegistryType.GitHub)
        {
            return "ghcr.io";
        }

        return RegistryHost;
    }
}